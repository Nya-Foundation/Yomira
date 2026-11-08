// One vector master; Tauri handles platform sizes and binary icon formats.
// Usage: bun run gen:icons (also refreshes an initialized Android project).
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const icons = join(root, 'src-tauri/icons');
const master = readFileSync(join(root, 'design/branding/yomira.svg'), 'utf8');
const mark = master.replace(/\s*<rect id="background"[^>]*\/>/, '');

// The adaptive foreground is inset here rather than through the manifest's
// `android_fg_scale`, which this CLI ignores outright — 40 and 75 both render
// the mark identically. Android's 66.7% safe zone is only what the mask
// guarantees; a near-square mark sits on Material's product-icon keyline at
// about 53% of the 108dp canvas. The mark fills 59.4% of the master's 512 box
// and the CLI renders that box to the full canvas, so widening the box around
// the same artwork is what gives the icon its breathing room.
const FOREGROUND_PAD = 1.125;
const inset = (svg) => {
  const box = Math.round(512 * FOREGROUND_PAD);
  const offset = (box - 512) / 2;
  return svg.replace(
    'width="512" height="512" viewBox="0 0 512 512"',
    `width="${box}" height="${box}" viewBox="${-offset} ${-offset} ${box} ${box}"`,
  );
};
const temp = mkdtempSync(join(tmpdir(), 'yomira-icons-'));
const require = createRequire(import.meta.url);
const cli = join(dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js');

try {
  writeFileSync(join(temp, 'icon.svg'), master);
  writeFileSync(join(temp, 'mark.svg'), inset(mark));
  writeFileSync(join(temp, 'monochrome.svg'), inset(mark).replace('#fb7185', '#ffffff'));
  writeFileSync(join(temp, 'icons.json'), JSON.stringify({
    default: 'icon.svg',
    bg_color: '#0a0a0c',
    android_fg: 'mark.svg',
    android_monochrome: 'monochrome.svg',
  }));
  execFileSync(process.execPath, [cli, 'icon', join(temp, 'icons.json'), '--output', icons], { cwd: root, stdio: 'inherit' });
  writeFileSync(join(root, 'public/yomira.svg'), master);
  writeFileSync(join(root, 'public/yomira-mark.svg'), mark);
  cpSync(join(icons, '128x128.png'), join(root, 'public/yomira.png'));

  // Tauri writes mobile resources into initialized platform projects automatically.
  const projectRes = join(root, 'src-tauri/gen/android/app/src/main/res');
  const androidRes = existsSync(projectRes) ? projectRes : join(icons, 'android');
  const adaptive = join(androidRes, 'mipmap-anydpi-v26');
  cpSync(join(adaptive, 'ic_launcher.xml'), join(adaptive, 'ic_launcher_round.xml'));
  console.log('Yomira Folded Y icons generated from design/branding/yomira.svg');
} finally {
  rmSync(temp, { recursive: true, force: true });
}
