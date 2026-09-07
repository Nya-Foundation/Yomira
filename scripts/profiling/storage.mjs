import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';

const output = resolve(process.argv[2] ?? 'artifacts/performance/storage.json');
const temporary = mkdtempSync(join(tmpdir(), 'exviewer-profiler-'));
try {
  const executable = join(temporary, process.platform === 'win32' ? 'storage.exe' : 'storage');
  execFileSync('rustc', ['--edition=2021', '-O', 'scripts/profiling/storage.rs', '-o', executable], { stdio: 'inherit' });
  const report = execFileSync(executable, { encoding: 'utf8' });
  JSON.parse(report);
  mkdirSync(resolve(output, '..'), { recursive: true });
  writeFileSync(output, report);
  console.log(report);
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
