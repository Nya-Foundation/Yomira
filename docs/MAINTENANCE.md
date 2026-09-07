# Architecture and maintenance

Yomira separates presentation, IPC contracts and native work. Extend the
existing domain modules before introducing another layer.

| Area | Responsibility |
| --- | --- |
| `src/pages/` | Route coordination, query state and return destinations |
| `src/components/` | Feature UI; shared layout and accessible primitives |
| `src/lib/api.ts`, `src/lib/api/` | Public command exports, typed wrappers and error normalization |
| `src/lib/types.ts` | DTOs shared across frontend consumers |
| `src/lib/mock.ts`, `src/lib/mock/` | Lazy browser-only demo fixtures and persistence |
| `src-tauri/src/commands/` | Tauri IPC handlers and DTO adaptation |
| `src-tauri/src/core/` | Client transport, DNS, cookies, models and HTML parsers |
| `src-tauri/src/db/` | SQLite persistence and migrations |
| `src-tauri/src/download/` | Persistent queue, spider, local files and offline reading |
| `src-tauri/src/tags/` | Translation database parsing, refresh and suggestions |
| `src-tauri/gen/android/` | Maintained Gradle project and Android back integration |

## Contracts and persistence

Add new frontend commands to the appropriate `api/` module and export them
through `api.ts`. Implement and register the matching snake_case Rust command.
Keep public DTOs stable, or update every producer, fixture and consumer together.
Only Rust opens the application database.

The application identifier, Android package name, `exviewer.db`, and legacy
browser storage keys preserve compatibility with existing installs. Renaming
them is a data migration, not branding cleanup.

Browser fixtures load dynamically outside Tauri. Native startup must not load
demo galleries or browser persistence helpers. Download events invalidate
reader metadata; idle polling is intentionally absent.

## Generated files

Commit both dependency lockfiles, the logo master and platform icon resources.
`bun run gen:icons` refreshes the icons. Keep the Android manifest,
`MainActivity.kt`, Gradle scripts, wrapper and resources under version control.
Tauri regenerates machine-specific Gradle bindings, JNI libraries and generated
Kotlin files; nested `.gitignore` files exclude them.

Build caches, local artifacts, environment files, signing keys and profiling
traces do not belong in Git. Reproducible profiling summaries under
`docs/performance/` are intentional documentation.

## Verification

CI runs TypeScript, frontend unit and browser tests, release-tooling tests,
Rust formatting, Clippy and Rust tests. Platform jobs compile installable
bundles. These do not prove live-site compatibility or physical-device behavior.
Use synthetic regression fixtures and record native manual checks separately.

Performance profiling is separate from correctness gates because host load
affects timing. See [PERFORMANCE.md](PERFORMANCE.md) for the fixed workload
and measurement limits.
