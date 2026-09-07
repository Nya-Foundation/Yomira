# Third-party notices

## EhViewer and Ehviewer_CN_SXJ

Yomira contains modified Rust ports of code from
[xiaojieonly/Ehviewer_CN_SXJ](https://github.com/xiaojieonly/Ehviewer_CN_SXJ),
building on the original EhViewer work by Hippo Seven and contributors.
The reference checkout for the port is the `BiLi_PC_Gamer` revision
[`2c451bf3c9e72bbf5088859b199929dd8ba4f6a1`](https://github.com/xiaojieonly/Ehviewer_CN_SXJ/tree/2c451bf3c9e72bbf5088859b199929dd8ba4f6a1).

That revision's root LICENSE contains interleaved GPL 3.0 and Apache 2.0 text.
The inspected Java/Kotlin client, parser, spider and tag database files carry
Apache-2.0 notices. This is not an assertion that every upstream file has the
same license, or that the root file offers a choice of licenses. Yomira retains
GPL-3.0-only for the combined application and preserves the upstream notices
and Apache terms for the adapted portions. An Apache-only relicensing of the
whole application would require a complete provenance review and clarification
of the upstream licensing ambiguity.

The following materials accompany the source and application bundles:

- [Original NOTICE](NOTICE), reproduced verbatim after Yomira's notice.
- [Apache License 2.0](licenses/Apache-2.0.txt).
- [Unmodified upstream LICENSE snapshot](licenses/Ehviewer_CN_SXJ-LICENSE.txt).

Adaptations include `src-tauri/src/core/` (client, models and parsers),
`src-tauri/src/download/` (queue/spider and `.ehviewer` storage behavior),
`src-tauri/src/tags/db.rs`, related command handlers, and
`src-tauri/resources/hosts.txt`. They have been translated and modified for
Rust, asynchronous I/O, Tauri IPC, SQLite persistence and cross-platform use.
They are not unmodified copies of the original Android application.

Original notices include Copyright 2014–2019 Hippo Seven; later upstream
contributors retain copyright in their respective changes.

## Tag translations

The application downloads the packed Chinese tag-translation database from
[Mapaler/EhViewer's tag-translations branch](https://github.com/Mapaler/EhViewer/tree/tag-translations)
at runtime. The database is not vendored in this repository. Its content and
distribution terms remain those of its upstream authors and data sources.

## Libraries

JavaScript and Rust dependencies remain under their respective licenses.
`bun.lock` and `src-tauri/Cargo.lock` record their resolved versions; installed
packages include their license files. Tauri, React, TanStack, Tokio, reqwest,
rusqlite, scraper, Lucide and the other dependencies are independent projects
and do not endorse Yomira.
