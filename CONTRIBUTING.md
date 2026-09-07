# Contributing to Yomira

Small, focused improvements are welcome. For a large feature or architectural
change, open an issue describing the user problem and proposed scope first.
The [roadmap](docs/ROADMAP.md) identifies unfinished workflows.

## Set up and verify

Follow the [README](README.md#develop-locally) and platform
[build instructions](docs/BUILDS.md). Use Bun and commit its lockfile when
dependencies change. Rust and Node versions are recorded in the repository.

Before submitting a pull request, run:

```sh
bun run check:repo
bun run build
bun run test:unit
bun run test:release
bunx playwright install chromium
bun run test:e2e
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Describe what changed, why, how you checked it, and any remaining limitations.
Include desktop and narrow-screen screenshots for visible UI changes, using
the synthetic browser demo. Never submit account cookies, private galleries,
signing keys, machine-specific Gradle files or build outputs.

## Code and design

Keep route coordination in pages, feature UI in its component directory,
typed IPC in the API domain modules, and network/filesystem work in Rust.
Prefer a direct implementation over an abstraction with one caller. Preserve
persisted identifiers and schemas unless the change includes a migration.
Parser changes need representative synthetic fixtures, including failure cases.

Follow the [design system](docs/DESIGN_SYSTEM.md). Check keyboard focus, touch
targets, loading/error states, both themes and narrow viewports. Add or update
English and Chinese strings together.

## Commit messages

Use Conventional Commits for pull-request titles (and commits when not squash
merging): `fix(reader): restore swipe navigation`, `feat(search): add a filter`,
`docs: improve setup instructions`. `fix` and `perf` trigger patch releases;
`feat` triggers a minor release; a `BREAKING CHANGE:` footer triggers a major
release. Other types do not trigger a release on their own. See
[Releasing](docs/RELEASING.md) for the complete process.

Contributions are accepted under the project's GPL-3.0-only license. Retain
existing third-party attribution and document the source and license of any
newly adapted code or assets.
