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
const temp = mkdtempSync(join(tmpdir(), 'yomira-icons-'));
const require = createRequire(import.meta.url);
const cli = join(dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js');

try {
  writeFileSync(join(temp, 'icon.svg'), master);
  writeFileSync(join(temp, 'mark.svg'), mark);
  writeFileSync(join(temp, 'monochrome.svg'), mark.replace('#fb7185', '#ffffff'));
  writeFileSync(join(temp, 'icons.json'), JSON.stringify({
    default: 'icon.svg',
    bg_color: '#0a0a0c',
    android_fg: 'mark.svg',
    android_fg_scale: 75,
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
