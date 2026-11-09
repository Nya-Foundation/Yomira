// Serve the production build with tauri.conf.json's CSP applied as a real
// response header, drive it in Chromium, and report every CSP violation.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { chromium } from '@playwright/test';

const ROOT = new URL('..', import.meta.url).pathname;
const DIST = join(ROOT, 'dist');
const conf = JSON.parse(await readFile(join(ROOT, 'src-tauri/tauri.conf.json'), 'utf8'));
const csp = conf.app.security.csp;
if (!csp) throw new Error('no csp configured');

const TYPES = {
  '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.json': 'application/json',
  '.woff2': 'font/woff2', '.ico': 'image/x-icon',
};

const server = createServer(async (req, res) => {
  const url = new URL(req.url, 'http://localhost');
  let file = normalize(url.pathname).replace(/^(\.\.[/\\])+/, '');
  if (file === '/' || !extname(file)) file = '/index.html';
  try {
    const body = await readFile(join(DIST, file));
    res.writeHead(200, {
      'Content-Type': TYPES[extname(file)] ?? 'application/octet-stream',
      // Exactly what Tauri would send for the app's own pages.
      'Content-Security-Policy': csp,
    });
    res.end(body);
  } catch {
    res.writeHead(404, { 'Content-Security-Policy': csp });
    res.end('not found');
  }
});

await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;

const violations = [];
const pageErrors = [];
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

// Real CSP enforcement surfaces here; console messages are the fallback signal.
await page.addInitScript(() => {
  document.addEventListener('securitypolicyviolation', (e) => {
    window.__csp ??= [];
    window.__csp.push({
      directive: e.effectiveDirective,
      blocked: String(e.blockedURI).slice(0, 120),
      sample: String(e.sample ?? '').slice(0, 80),
    });
  });
});
page.on('pageerror', (e) => pageErrors.push(String(e).slice(0, 200)));
page.on('console', (m) => {
  const t = m.text();
  if (/Content Security Policy|Refused to/i.test(t)) violations.push(t.slice(0, 200));
});

// Block anything leaving the machine; remote comment emotes are not the subject.
await page.route('**', (route) => {
  const u = route.request().url();
  if (u.startsWith(base) || u.startsWith('data:') || u.startsWith('blob:')) return route.continue();
  return route.abort();
});

const steps = [];
async function visit(label, path, ready) {
  await page.goto(`${base}${path}`, { waitUntil: 'load' });
  let ok = true;
  try {
    await page.waitForSelector(ready, { timeout: 15000 });
  } catch {
    ok = false;
  }
  const seen = await page.evaluate(() => window.__csp ?? []);
  steps.push({ label, rendered: ok, violations: seen.length });
  if (seen.length) violations.push(...seen.map((v) => `${label}: ${v.directive} blocked ${v.blocked} ${v.sample}`));
  await page.evaluate(() => { window.__csp = []; });
}

// The shell, a grid of images, a detail screen with comments, and settings.
await visit('home', '/', 'img, [role="img"]');
await visit('settings', '/settings', 'h1, h2, [role="tablist"]');
await visit('downloads', '/downloads', 'main');

// Inline styles and portalled dialogs are the classic style-src tripwires.
await page.goto(`${base}/`, { waitUntil: 'load' });
await page.waitForSelector('img, [role="img"]');
await page.keyboard.press('Control+k');
const dialog = await page.waitForSelector('dialog[open]', { timeout: 5000 }).then(() => true).catch(() => false);
const afterDialog = await page.evaluate(() => window.__csp ?? []);
if (afterDialog.length) violations.push(...afterDialog.map((v) => `palette: ${v.directive} blocked ${v.blocked}`));

await browser.close();
server.close();

console.log('policy:', csp, '\n');
for (const s of steps) console.log(`  ${s.rendered ? 'rendered' : 'BLANK   '}  ${s.label}  violations=${s.violations}`);
console.log(`  ${dialog ? 'rendered' : 'BLANK   '}  command palette (portalled dialog + inline styles)`);
console.log('\npage errors:', pageErrors.length ? pageErrors : 'none');
console.log('csp violations:', violations.length ? violations : 'none');
process.exit(violations.length || pageErrors.length || steps.some((s) => !s.rendered) || !dialog ? 1 : 0);
