import { installFixture } from './fixture.mjs';
// Production-renderer profile with deterministic IPC fixtures. This excludes
// native process startup, real network latency and real-image decoding.
import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile, stat } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { platform, arch, cpus } from 'node:os';

const output = resolve(process.argv[2] ?? 'artifacts/performance/ui');
const runs = Number(process.env.PROFILE_RUNS ?? 5);
const dist = resolve('dist');
await mkdir(output, { recursive: true });
const server = createServer(async (req, res) => {
  try {
    const pathname = new URL(req.url, 'http://localhost').pathname;
    let path = resolve(dist, `.${pathname}`);
    if (!path.startsWith(dist + sep)) path = resolve(dist, 'index.html');
    try { if (!(await stat(path)).isFile()) path = resolve(dist, 'index.html'); }
    catch { path = resolve(dist, 'index.html'); }
    const mime = { '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.html': 'text/html', '.svg': 'image/svg+xml' };
    res.writeHead(200, { 'Content-Type': mime[extname(path)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
    res.end(await readFile(path));
  } catch { res.writeHead(500).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch();


async function snapshot(cdp) {
  const {metrics} = await cdp.send('Performance.getMetrics');
  const values = Object.fromEntries(metrics.map(m => [m.name, m.value]));
  return { taskMs: values.TaskDuration * 1000, scriptMs: values.ScriptDuration * 1000, layoutMs: values.LayoutDuration * 1000, nodes: values.Nodes, heapBytes: values.JSHeapUsedSize };
}
const delta = (before, after) => Object.fromEntries(Object.keys(after).map(key=>[key, after[key] - before[key]]));
const percentile = (values, fraction) => [...values].sort((a,b)=>a-b)[Math.min(values.length-1, Math.floor(values.length*fraction))];
const samples = [];
try {
  for (let run = 0; run < runs; run++) {
    const context = await browser.newContext({viewport:{width:1440,height:900}});
    await context.addInitScript(installFixture);
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await cdp.send('Performance.enable');
    await cdp.send('Profiler.enable');
    await cdp.send('Profiler.start');
    await page.goto(origin);
    await page.locator('main a[href^="/gallery/"] img').first().waitFor();
    await page.locator('main a[href^="/gallery/"] img').first().evaluate(img=>img.decode());
    const readyMs = await page.evaluate(()=>performance.now());
    const cold = await snapshot(cdp);
    const resources = await page.evaluate(()=>performance.getEntriesByType('resource').filter(r=>new URL(r.name).pathname.endsWith('.js')).map(r=>({file:new URL(r.name).pathname, bytes:r.decodedBodySize})));
    const scrollBefore = await snapshot(cdp);
    const frames = await page.locator('main .overflow-y-auto').evaluate(async element => {
      const gaps = []; let previous = performance.now();
      for (let i=0;i<180;i++) {
        await new Promise(requestAnimationFrame);
        const now=performance.now(); gaps.push(now-previous); previous=now;
        element.scrollTop += 70;
      }
      return gaps;
    });
    const scroll = delta(scrollBefore, await snapshot(cdp));
    const cards = await page.locator('main a[href^="/gallery/"]').count();
    const listCalls = await page.evaluate(()=>window.__profileCalls.get_gallery_list ?? 0);
    // Direct reader startup is measured separately from home startup.
    await page.goto(`${origin}/read/1/fixture?source=download`);
    await page.getByRole('img', {name:'Page 1', exact:true}).waitFor();
    const readerReadyMs = await page.evaluate(()=>performance.now());
    await cdp.send('HeapProfiler.collectGarbage');
    const readerBefore = await snapshot(cdp);
    const turns = [];
    for (let index=1; index<=30; index++) {
      turns.push(await page.evaluate(async index => {
        const start=performance.now();
        window.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}));
        const image = await new Promise((resolve, reject) => {
          const timeout = setTimeout(() => { observer.disconnect(); reject(new Error("Reader page did not render within 5 seconds")); }, 5000);
          const find = () => document.querySelector(`img[alt="Page ${index+1}"]`);
          const observer = new MutationObserver(()=>{ const img=find(); if(img){observer.disconnect();clearTimeout(timeout);resolve(img);} });
          observer.observe(document.querySelector('main'),{subtree:true,childList:true,attributes:true});
          const img=find();if(img){observer.disconnect();clearTimeout(timeout);resolve(img);}
        });
        await image.decode(); await new Promise(requestAnimationFrame);
        return performance.now()-start;
      },index));
    }
    await page.getByRole('button',{name:'Toggle reading mode'}).click();
    await page.getByRole('img',{name:'Page 31',exact:true}).waitFor();
    const continuousImages = await page.getByRole('img').count();
    // Five-second window catches periodic reader work, if any.
    const idleBefore = await snapshot(cdp);
    const callsBefore = await page.evaluate(()=>({...window.__profileCalls}));
    await page.waitForTimeout(5200);
    const idle = delta(idleBefore, await snapshot(cdp));
    const callsAfter = await page.evaluate(()=>({...window.__profileCalls}));
    await cdp.send('HeapProfiler.collectGarbage');
    const readerAfter = await snapshot(cdp);
    const {profile} = await cdp.send('Profiler.stop');
    await writeFile(resolve(output, `run-${run+1}.cpuprofile`), JSON.stringify(profile));
    if(errors.length) throw new Error(errors.join('\n'));
    const sample = {readyMs,readerReadyMs,cold,jsBytes:resources.reduce((sum,r)=>sum+r.bytes,0),resources,scroll:{...scroll,frameP95Ms:percentile(frames,.95),framesOver50Ms:frames.filter(n=>n>50).length,cards,listCalls},reader:{turnP50Ms:percentile(turns,.5),turnP95Ms:percentile(turns,.95),retainedHeapDeltaBytes:readerAfter.heapBytes-readerBefore.heapBytes,continuousImages,idleTaskMs:idle.taskMs,idleMetadataCalls:(callsAfter.get_downloaded_gallery??0)-(callsBefore.get_downloaded_gallery??0)}};
    samples.push(sample);
    console.log(`Run ${run+1}: ready ${readyMs.toFixed(1)} ms, JS ${sample.jsBytes} bytes, scroll p95 ${sample.scroll.frameP95Ms.toFixed(1)} ms, turn p95 ${sample.reader.turnP95Ms.toFixed(1)} ms`);
    await context.close();
  }
  const summary = { environment:{os:platform(),arch:arch(),cpu:cpus()[0]?.model,chromium:browser.version(),viewport:'1440x900',runs,fixture:'deterministic IPC; SVG pages; no real network or native startup'},medians:{readyMs:percentile(samples.map(s=>s.readyMs),.5),jsBytes:percentile(samples.map(s=>s.jsBytes),.5),coldTaskMs:percentile(samples.map(s=>s.cold.taskMs),.5),scrollTaskMs:percentile(samples.map(s=>s.scroll.taskMs),.5),scrollFrameP95Ms:percentile(samples.map(s=>s.scroll.frameP95Ms),.5),turnP95Ms:percentile(samples.map(s=>s.reader.turnP95Ms),.5),idleMetadataCalls:percentile(samples.map(s=>s.reader.idleMetadataCalls),.5)},samples};
  await writeFile(resolve(output,'summary.json'),JSON.stringify(summary,null,2));
  await writeFile(resolve(output,'bundle-profile.json'),await readFile(resolve(dist,'bundle-profile.json')));
  console.log(JSON.stringify(summary.medians,null,2));
} finally { await browser.close(); await new Promise(resolve=>server.close(resolve)); }
