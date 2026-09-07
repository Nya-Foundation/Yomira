import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function validateVersion(version) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error('Expected a stable version such as 1.2.3');
  }
  const [major, minor, patch] = version.split('.').map(Number);
  if (minor > 999 || patch > 999 || major * 1_000_000 + minor * 1_000 + patch > 2_100_000_000) {
    throw new Error('Version exceeds the Android versionCode limits');
  }
  return version;
}

export function syncVersion(version, root = process.cwd()) {
  validateVersion(version);
  const files = ['package.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock'];
  const contents = files.map(file => readFileSync(resolve(root, file), 'utf8'));
  const patterns = [null, null, /(\[package\][\s\S]*?\nversion = ")([^"]+)(")/,
    /(\[\[package\]\]\nname = "yomira"\nversion = ")([^"]+)(")/];
  const versions = contents.map((text, index) => {
    if (index < 2) return JSON.parse(text).version;
    const match = text.match(patterns[index]);
    if (!match) throw new Error(`Missing Yomira version in ${files[index]}`);
    return match[2];
  });
  if (new Set(versions).size !== 1) throw new Error(`Versions disagree: ${versions.join(', ')}`);
  // Validate every input before modifying any file.
  const updated = contents.map((text, index) => {
    if (index < 2) return JSON.stringify({ ...JSON.parse(text), version }, null, 2) + '\n';
    return text.replace(patterns[index], (_, before, _old, after) => before + version + after);
  });
  files.forEach((file, index) => writeFileSync(resolve(root, file), updated[index]));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  syncVersion(process.argv[2]);
}
