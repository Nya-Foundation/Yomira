import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { syncVersion, validateVersion } from '../scripts/release/version.mjs';
import { collectAssets, expectedAssets, verifyAssets } from '../scripts/release/assets.mjs';
import { verifyRelease } from '../scripts/release/plugin.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'yomira-release-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'src-tauri'));
  for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
    writeFileSync(join(root, file), JSON.stringify({ version: '0.2.2', name: 'yomira' }));
  }
  writeFileSync(join(root, 'src-tauri/Cargo.toml'), '[package]\nname = "yomira"\nversion = "0.2.2"\n');
  writeFileSync(join(root, 'src-tauri/Cargo.lock'), '[[package]]\nname = "other"\nversion = "0.2.2"\n\n[[package]]\nname = "yomira"\nversion = "0.2.2"\n');
  return root;
}

test('version stamping updates all manifests without changing dependency versions', t => {
  const root = fixture(t);
  syncVersion('1.2.3', root);
  syncVersion('1.2.3', root);
  assert.equal(JSON.parse(readFileSync(join(root, 'package.json'))).version, '1.2.3');
  assert.equal(JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'))).version, '1.2.3');
  assert.match(readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8'), /version = "1.2.3"/);
  const lock = readFileSync(join(root, 'src-tauri/Cargo.lock'), 'utf8');
  assert.match(lock, /name = "other"\nversion = "0.2.2"/);
  assert.match(lock, /name = "yomira"\nversion = "1.2.3"/);
});

test('invalid versions and inconsistent manifests fail before any writes', t => {
  const root = fixture(t);
  const before = readFileSync(join(root, 'package.json'), 'utf8');
  for (const version of ['01.2.3', '1.0.0-beta.1', '1.1000.0', '1.0.1000', '99999.0.0', '', '1.2.3\n']) {
    assert.throws(() => validateVersion(version));
  }
  writeFileSync(join(root, 'src-tauri/tauri.conf.json'), '{"version":"9.0.0"}');
  assert.throws(() => syncVersion('1.0.0', root), /disagree/);
  assert.equal(readFileSync(join(root, 'package.json'), 'utf8'), before);
});

test('publication rejects partial, empty and wrong-version assets and produces checksums', t => {
  const root = fixture(t);
  const assets = join(root, 'release-assets');
  mkdirSync(assets);
  assert.throws(() => verifyAssets(assets, '1.0.0'), /do not match/);
  for (const name of expectedAssets('1.0.0')) writeFileSync(join(assets, name), 'fixture');
  assert.throws(() => verifyAssets(assets, '1.0.1'), /do not match/);
  verifyAssets(assets, '1.0.0');
  assert.equal(readFileSync(join(assets, 'SHA256SUMS'), 'utf8').trim().split('\n').length, 6);
  writeFileSync(join(assets, expectedAssets('1.0.0')[0]), '');
  assert.throws(() => verifyAssets(assets, '1.0.0'), /Empty/);
  assert.throws(() => verifyRelease({}, { nextRelease: { version: '1.0.1' }, env: { RELEASE_VERSION: '1.0.0' }, cwd: root }), /changed/);
});

test('collection fails on missing or ambiguous packages', t => {
  const root = fixture(t);
  const source = join(root, 'bundle');
  mkdirSync(source);
  const output = join(root, 'out');
  assert.throws(() => collectAssets('windows-x64', source, output, '1.0.0'), /found 0/);
  writeFileSync(join(source, 'setup.exe'), 'fixture');
  collectAssets('windows-x64', source, output, '1.0.0');
  assert.equal(readFileSync(join(output, 'Yomira-1.0.0-windows-x64.exe'), 'utf8'), 'fixture');
  writeFileSync(join(source, 'old-setup.exe'), 'stale');
  assert.throws(() => collectAssets('windows-x64', source, output, '1.0.0'), /found 2/);
});
