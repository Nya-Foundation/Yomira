import { execFileSync } from 'node:child_process';
import { existsSync, lstatSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const files = [...new Set(execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard'],
  { encoding: 'utf8' }).split('\0').filter(Boolean))];
const errors = [];
const forbidden = /(^|\/)(node_modules|target|dist|artifacts|test-results|playwright-report|\.gradle|\.idea|\.agents)(\/|$)|(^|\/)\.env(?!\.example$)(\.|$)|\.(jks|keystore|p12|pfx|pem|apk|aab|exe|dmg|AppImage|db|tsbuildinfo)$|(^|\/)(local\.properties|tauri\.settings\.gradle|tauri\.build\.gradle\.kts|tauri\.properties|cookies\.json|skills-lock\.json)$/;
for (const file of files) {
  if (!existsSync(file)) continue; // Pending deletions are not publishable files.
  if (forbidden.test(file) || /\/java\/.*\/generated\//.test(file) || /\/jniLibs\//.test(file)) {
    errors.push(`${file}: local/generated/private file must not be tracked`);
    continue;
  }
  const stat = lstatSync(file);
  if (stat.isSymbolicLink()) errors.push(`${file}: source checkout must not depend on local symlinks`);
  if (stat.size > 2 * 1024 * 1024) errors.push(`${file}: exceeds the 2 MiB source-file budget`);
  if (/\.(png|ico|icns|jar)$/.test(file)) continue;
  const content = readFileSync(file, 'utf8');
  if (/(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{60,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)/.test(content)) {
    errors.push(`${file}: possible credential (value omitted)`);
  }
  if (file.endsWith('.md')) {
    const links = [...content.matchAll(/\]\(([^\s)]+)(?:\s+"[^"]*")?\)|(?:src|href)="([^"]+)"/g)];
    for (const match of links) {
      const link = match[1] ?? match[2];
      if (/^(?:[a-z]+:|#|\/\/)/i.test(link)) continue;
      const path = decodeURIComponent(link.split('#')[0]);
      if (path && !existsSync(resolve(dirname(file), path))) errors.push(`${file}: broken link ${link}`);
    }
  }
}
const versions = [JSON.parse(readFileSync('package.json')).version, JSON.parse(readFileSync('src-tauri/tauri.conf.json')).version,
  readFileSync('src-tauri/Cargo.toml', 'utf8').match(/\[package\][\s\S]*?\nversion = "([^"]+)"/)?.[1],
  readFileSync('src-tauri/Cargo.lock', 'utf8').match(/\[\[package\]\]\nname = "yomira"\nversion = "([^"]+)"/)?.[1]];
if (new Set(versions).size !== 1 || !versions[0]) errors.push('Application versions disagree');
if (errors.length) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Repository hygiene passed: ${files.length} source files; versions agree at ${versions[0]}`);
}
