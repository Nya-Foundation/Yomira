# Releasing Yomira

## One-time GitHub setup

1. Push the repository to GitHub with `main` as its default branch.
2. Add the repository Actions secrets `ANDROID_KEYSTORE_BASE64`,
   `ANDROID_KEY_ALIAS`, `ANDROID_KEYSTORE_PASSWORD` and `ANDROID_KEY_PASSWORD`.
   Keep the keystore outside Git and back it up securely. See
   [Android signing](BUILDS.md#android) and
   [Tauri's signing guide](https://v2.tauri.app/distribute/sign/android/).
3. Set the repository Actions variable `RELEASE_ENABLED` to `true` when ready
   to distribute packages. Until then, CI builds all platforms and automatic
   publishing is skipped.
4. Allow the release job's `GITHUB_TOKEN` to write repository contents. If main
   is protected, the release bot needs permission to push its version commit;
   adapt your ruleset or use a narrowly scoped GitHub App token. Do not give
   write credentials to pull-request workflows.

No npm token is needed: this is an application, not a published npm package.
Desktop signing certificates are not yet configured. Android release signing
is mandatory; unsigned CI APKs are build artifacts, not installable releases.

## Android release signing identity

The public release certificate uses RSA-4096. Its SHA-256 fingerprint is:

```text
f8f48add1dec22298d055e320ae37a1d8c28948b07c81d1011d9069aea978447
```

Use `apksigner verify --print-certs <apk>` to compare an APK's signer with this
fingerprint. The private PKCS12 keystore and passwords are kept outside the
repository and supplied to the release workflow through Actions secrets.

This identity differs from the earlier local test-build key. Those installations
cannot update directly to release APKs; switching requires uninstalling the test
build first, which removes its private app data. Preserve anything you need
before switching. Future release APKs use this same release identity.

## Version policy

[semantic-release](https://github.com/semantic-release/semantic-release)
reads Conventional Commits on `main` since the previous `v*` release tag:

| Commit | Release |
| --- | --- |
| `fix: ...`, `perf: ...` | Patch |
| `feat: ...` | Minor |
| `BREAKING CHANGE: ...` footer | Major |
| `docs: ...`, `chore: ...`, `refactor: ...`, `test: ...`, `ci: ...` | None alone |

Use squash-merge PR titles that follow this convention. To signal a breaking
change reliably, include a `BREAKING CHANGE:` footer in the merged commit.
The first automated release is **1.0.0** when no prior release tag exists;
`CHANGELOG.md` records earlier local development versions without inventing
historical Git releases. If maintaining a pre-1.0 series, create a deliberate
baseline tag on the corresponding source commit before enabling releases.

Stable versions must fit Android's version-code scheme: minor and patch are
0–999 and the resulting code must not exceed 2,100,000,000. Prerelease channels
are not configured.

## Pipeline

1. On a main push or manual dispatch, plan the next version without publishing.
2. Run repository hygiene, frontend, browser and Rust checks.
3. Build Windows x64, Linux x64, macOS ARM64, macOS x64 and Android ARM packages
   from the triggering commit. Stamp the planned version into `package.json`,
   `Cargo.toml`, `Cargo.lock` and `tauri.conf.json` before each build.
4. Sign and verify the universal Android APK. No signing key is available to PR
   builds, and the key is materialized only in a temporary signing directory.
5. Require every build to succeed. Check that main still points to the tested
   commit, recompute the release version, and reject missing, stale, empty or
   unexpected packages. Generate `SHA256SUMS`.
6. Commit the same version changes with `[skip ci]`, tag that commit and publish
   the six packages and checksums to GitHub Releases. Release notes come from
   commits. GitHub's source archives point to the versioned source commit,
   including the lockfiles, license notices and build scripts.

The release workflow serializes publication; it does not cancel an active
release. If main advances while a build runs, that run stops before publication
and the queued newer run builds the new source. The normal CI workflow cancels
superseded runs. Third-party Actions are pinned to commit SHAs and Dependabot
checks those pins monthly.

`bun run release:plan` runs a dry run and needs a Git remote and appropriate
repository access. `bun run release` publishes and is intended for the final
workflow job, after artifacts are available. `bun run test:release` checks
version synchronization and incomplete-release failure cases locally.

## Recovery

If a check or build fails, fix it and rerun; nothing has been published yet.
A failure while uploading to GitHub can leave a tag or partial release because
remote publication is not transactional. Inspect the existing release before
retrying. Do not delete or move a public release tag silently. For a published
bad package, fix the problem and issue a new version. For an interrupted upload,
finish attaching the already built, verified files to that same release.

Releases do not yet configure an in-app updater, Windows Authenticode, Apple
Developer ID/notarization, store uploads or iOS builds.
