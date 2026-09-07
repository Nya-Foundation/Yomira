# Performance baseline and cleanup — 2026-09-07

The first pass measured the production renderer with deterministic IPC fixtures and the actual Rust download-storage implementation. It found avoidable startup payload and recurring storage work, while the measured scrolling and page-turn flows remained responsive. The maintainability changes are described in [MAINTENANCE.md](MAINTENANCE.md).

## Measured results

Linux x64, AMD Ryzen 7 9800X3D, headless Chromium 151.0.7922.34, 1440×900 viewport. Renderer figures are medians of five fresh browser contexts per version, with CPU sampling enabled in both versions. No browser or CPU throttling was applied.

| Renderer measurement | Before | After | Interpretation |
| --- | ---: | ---: | --- |
| JavaScript loaded before the initial gallery image | 456,631 B | 436,596 B | 20,035 B less (4.4%); demo modules are absent from native-path requests |
| Automation-observed gallery-ready time | 866.8 ms | 870.1 ms | No demonstrated startup speedup |
| Main-thread task time through gallery readiness | 145.3 ms | 134.4 ms | Modest change; scheduling-sensitive |
| Main-thread task time over the fixed scroll | 588.9 ms | 604.4 ms | No demonstrated scroll CPU improvement; individual before/after ranges overlap |
| Scroll frame-gap p95 | 18.6 ms | 18.6 ms | Essentially unchanged for this headless workload |
| Page-turn p95, including synthetic image decode and next frame | 16.0 ms | 16.0 ms | Unchanged |
| Metadata requests during 5.2 seconds of idle offline reading | 1 | 0 | Polling removed; download events, focus and Retry trigger refresh |

Both versions issued nine list requests (225 fixture galleries) and retained 42 mounted gallery cards at the end of the scroll. No measured scroll frame exceeded 50 ms. This confirms that virtualization remains bounded in this workload; it is not a long-session leak test.

The Rust microbenchmark compiles the production `download/den.rs` with `rustc -O`. It creates fresh temporary folders, warms operations, then averages 1,000 header/page lookups or 30 scans. Filesystem caches are warm; no system-wide cache flush is attempted.

| Stored pages | Header read before | Header read after | Full scan before | Full scan after |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 3.286 μs | 2.770 μs | 0.151 ms | 0.149 ms |
| 1,000 | 5.347 μs | 2.963 μs | 1.489 ms | 1.467 ms |
| 10,000 | 23.002 μs | 2.981 μs | 16.129 ms | 14.739 ms |

At 10,000 pages, header-only access is about 7.7× faster because it no longer reads the 379 KB page-token tail. Full scans remain proportional to folder size; small timing differences here should not be credited as a scan algorithm improvement. The reader now avoids running those scans on an idle timer. Individual page lookup remains roughly 1–2 μs in these fixtures.

Removing the unused SQL plugin path and unused direct dependencies reduced Cargo lockfile package/version entries from 614 to 544. No dependency versions were upgraded. This is a dependency-graph reduction; native binary size and process startup were not measured.

## Reproduce

From the project root, with dependencies and Playwright Chromium installed:

```sh
bun run build:profile
bun run profile:ui artifacts/performance/my-run-ui
bun run profile:storage artifacts/performance/my-run-storage.json
```

`PROFILE_RUNS` optionally changes the renderer repeat count. The default is five. Run comparisons on the same machine without concurrent builds/tests, retain the raw samples, and compare distributions rather than a single fast run.

The UI runner serves the production build on a temporary loopback port, installs the IPC fixture, and measures:

1. A fresh home screen through the first decoded gallery image.
2. 180 animation frames of gallery scrolling at 70 px per frame.
3. Offline-reader startup, 30 page turns, a switch to continuous mode, and a 5.2-second idle window.
4. Garbage-collected JavaScript heap snapshots before and after that reader sequence.

The runner writes `summary.json`, per-run `.cpuprofile` captures, and Rollup’s module-size report. The `profile` build emits module-size information; ordinary production builds do not. CPU captures can be opened in Chromium DevTools; they contain minified production function names. The bundle report provides source-module attribution. The runner closes its browser/server and the storage benchmark removes its own fixtures.

Tracked summaries: [renderer before](performance/baseline-ui.json), [renderer after](performance/after-ui.json), [storage before](performance/baseline-storage.json), [storage after](performance/after-storage.json). Full local captures remain under `artifacts/performance/`, which is ignored by source control.

## Decisions and limits

The retained changes remove unused payload, idle work and duplicate infrastructure. A separate gallery-card memoization experiment increased scroll cost in this run and was reverted; no speculative memoization remains.

These measurements use synthetic IPC and SVG images. They do not measure Tauri process startup, the real IPC bridge, HTTP/site latency, native image decoding, GPU memory, battery use, Windows/WebView2, or macOS/WebKit. The storage fixtures contain small arbitrary byte payloads and measure metadata/filesystem operations only. Browser heap samples include normal query-cache retention and are not enough to establish long-term memory behavior.

Next useful native measurements are packaged cold/warm startup; page turns with realistic image sizes; a large library on slower or removable storage; and long reader sessions with cache eviction. The source review also found that `getFavoriteNote` still walks paginated favorites until it finds a gallery. That can produce substantial network latency for large accounts, but this pass did not benchmark authenticated site traffic or change its semantics.
