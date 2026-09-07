// Portions adapted from EhViewer / Ehviewer_CN_SXJ.
// Copyright 2014-2019 Hippo Seven and upstream contributors.
// Original portions: Apache-2.0; see licenses/Apache-2.0.txt and NOTICE.
// Modified for Yomira: Rust/Tauri port, async I/O and application integration.

//! Downloads engine — the Phase-2 port of EhViewer's
//! `SpiderQueen` / `SpiderDen` / `DownloadManager` trio
//! (github.com/xiaojieonly/Ehviewer_CN_SXJ, branch BiLi_PC_Gamer).
//!
//! Instead of the original's lock-based state machine, the engine is:
//! * [`manager::DownloadManager`] — the queue (persisted in the existing
//!   `downloads` / `download_labels` tables) plus a small fixed pool of tokio
//!   worker tasks. Each active gallery is processed by exactly one worker,
//!   sequentially, in page order; queue position defines global priority.
//! * [`spider::run_gallery`] — the per-gallery page pipeline (port of
//!   `SpiderQueen`'s download mode): resolve the page token, fetch the
//!   gallery page, follow H@H redirects, download the image bytes and hand
//!   them to the den. Network access goes through [`spider::PageSource`] so
//!   tests run without a network.
//! * [`den::GalleryDen`] — per-gallery folder `{download_root}/{dirname}`
//!   plus the `.ehviewer` metadata file (port of `SpiderInfo`), which stays
//!   byte-compatible with the original format so foreign galleries remain
//!   readable.
//!
//! State changes are pushed to the frontend through the Tauri event
//! [`manager::CHANGED_EVENT`] (`download://changed`); the frontend is
//! expected to refetch `list_downloads` on every emit.

pub mod den;
pub mod manager;
pub mod offline;
pub mod spider;
pub mod store;

pub use manager::{
    DownloadItem, DownloadManager, DownloadState, GalleryDownloadInfo, CHANGED_EVENT, WORKER_COUNT,
};
pub use spider::{EhPageSource, PageSource};
