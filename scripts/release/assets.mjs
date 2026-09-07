import { cpSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { validateVersion } from './version.mjs';

const formats = {
  'windows-x64': ['exe'],
  'linux-x64': ['deb', 'AppImage'],
  'macos-arm64': ['dmg'],
  'macos-x64': ['dmg'],
  'android-arm': ['apk'],
};

export function expectedAssets(version) {
  validateVersion(version);
  return Object.entries(formats).flatMap(([platform, extensions]) =>
    extensions.map(extension => `Yomira-${version}-${platform}.${extension}`)).sort();
}

export function verifyAssets(directory, version) {
  const expected = expectedAssets(version);
  const actual = readdirSync(directory).filter(name => name !== 'SHA256SUMS').sort();
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    throw new Error(`Release assets do not match ${version}. Expected: ${expected.join(', ')}; found: ${actual.join(', ')}`);
  }
  const sums = expected.map(name => {
    const bytes = readFileSync(join(directory, name));
    if (!bytes.length) throw new Error(`Empty release asset: ${name}`);
    return `${createHash('sha256').update(bytes).digest('hex')}  ${name}`;
  });
  writeFileSync(join(directory, 'SHA256SUMS'), sums.join('\n') + '\n');
}

function walk(directory) {
  return readdirSync(directory).flatMap(name => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

export function collectAssets(platform, source, destination, version) {
  validateVersion(version);
  if (!formats[platform]) throw new Error(`Unknown platform: ${platform}`);
  const files = walk(source);
  const copies = formats[platform].map(extension => {
    const matches = files.filter(path => path.endsWith(`.${extension}`));
    if (matches.length !== 1) throw new Error(`Expected one ${extension} in ${source}, found ${matches.length}`);
    return [matches[0], `Yomira-${version}-${platform}.${extension}`];
  });
  mkdirSync(destination, { recursive: true });
  for (const [sourcePath, name] of copies) cpSync(sourcePath, join(destination, name));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [platform, source, destination] = process.argv.slice(2);
  const version = JSON.parse(readFileSync('package.json', 'utf8')).version;
  collectAssets(platform, source, destination, version);
}
