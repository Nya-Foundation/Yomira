import { appendFileSync } from 'node:fs';
import semanticRelease from 'semantic-release';
import { validateVersion } from './version.mjs';

// Planning never creates a tag, modifies versions or publishes an asset.
const result = await semanticRelease({
  dryRun: true,
  plugins: ['@semantic-release/commit-analyzer', '@semantic-release/release-notes-generator'],
});
const version = result ? validateVersion(result.nextRelease.version) : '';
if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\nrelease=${Boolean(result)}\n`);
}
console.log(version ? `Planned release: ${version}` : 'No release needed');
