# Yomira roadmap

Priorities based on the current implementation and the original EhViewer workflows.

See [CHANGELOG.md](../CHANGELOG.md) for the initial development milestones.

What already ships is listed in the [README](../README.md); the interface
conventions behind it are in [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md). Local
favorites, MyTags editing and data migration are **not** complete. A database
table, parser or placeholder command does not constitute a finished feature.

## Extend the offline library

What offline reading already does, and the constraints it works under, are in
[OFFLINE_READING.md](OFFLINE_READING.md). Remaining:

- Rich offline gallery details (tags, comments and other metadata), rather than just reader metadata.
- Optional checksum/decode verification and explicit repair tools for corrupted images.
- Store reading progress in a versioned shared repository; surface it in detail/history/library.
- Add native folder selection/open-folder and a safe strategy for existing files when the destination changes.

## Make discovery and organization complete

- Restore whole saved-search objects, including categories/rating/page limits.
- Add local favorites, online favorite names/search and batch move/delete.
- Expose toplist timespan selection.
- Add download search, status/category filters, sorting and priority.
- Provide MyTags/site settings access with the existing session.

Acceptance: users can save a specific search, reopen its exact criteria, organize results and retrieve them later without remembering syntax or numeric collection slots.

## Improve reading comfort

- Fit width/height/original size, fullscreen and native keep-awake. RTL and swipe navigation are already available.
- Optional spreads, per-gallery overrides, image save/share and page bookmarks.
- Unified prefetch limits and quota-aware coordination with downloads.
- Profile pointer transforms, long sessions and image decoding before changing rendering architecture.

Acceptance: paged and continuous modes preserve the same position through resize, settings changes and restart, with consistent controls on keyboard and touch.

## Portability and release readiness

- Versioned backup/import of app data, excluding credentials by default.
- Import existing folders and local archives; verify upstream `.ehviewer` interoperability with real fixtures.
- An Android migration path, with optional LAN transfer after file import is reliable.
- Redacted diagnostics export, documented storage paths, cache controls.
- Signed desktop releases/update flow and actual native smoke tests on all three desktop platforms.
- Android device validation and platform-specific behavior; an iOS project and device checks remain outstanding.
- Rust formatting drift is resolved; preserve the formatting, type, unit-test and browser-test gates.

## Differentiators after the core is reliable

Continue-reading shelf, smart local collections, search scoped to online/library/history, private page bookmarks, optional metadata sync and desktop URL/file-opening conveniences.

The first production-renderer and optimized storage measurements are recorded in [PERFORMANCE.md](PERFORMANCE.md). Further performance claims need measurements: cold/warm startup, first usable grid, page-turn latency, scroll frame times, memory growth and request counts under fixed workloads. Framework choice is not a benchmark.
