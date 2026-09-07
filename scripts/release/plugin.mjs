import { syncVersion } from './version.mjs';
import { verifyAssets } from './assets.mjs';

export function verifyRelease(_config, { nextRelease, env, cwd }) {
  if (env.RELEASE_VERSION !== nextRelease.version) {
    throw new Error('Release version changed after planning; rebuild before publishing');
  }
  verifyAssets(`${cwd}/release-assets`, nextRelease.version);
}

export function prepare(_config, { nextRelease, cwd }) {
  syncVersion(nextRelease.version, cwd);
}
