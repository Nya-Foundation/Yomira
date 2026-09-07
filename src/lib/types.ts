export const CATEGORIES = [
  "Doujinshi",
  "Manga",
  "Artist CG",
  "Game CG",
  "Western",
  "Non-H",
  "Image Set",
  "Cosplay",
  "Asian Porn",
  "Misc",
] as const;

export type Category = (typeof CATEGORIES)[number];

export interface GalleryInfo {
  gid: number;
  token: string;
  title: string;
  titleJpn: string | null;
  category: Category;
  uploader: string;
  posted: number; // epoch seconds
  pages: number;
  rating: number; // 0..5
  ratedCount: number;
  favorited: number;
  language: string;
  thumb: string;
  size: number; // bytes
  torrentCount: number;
  fileCount: number;
}

export type ListPage = number | string;

export interface GalleryListResponse {
  galleries: GalleryInfo[];
  nextPage: ListPage | null;
  total: number;
  /** Total page count of this listing, 0 when unknown. */
  pages: number;
}

export interface TagGroup {
  namespace: string;
  tags: string[];
}

export interface Comment {
  id: number;
  author: string;
  score: number;
  time: number; // epoch seconds
  /** Epoch seconds of the last edit, 0 when never edited. */
  lastEdited: number;
  text: string;
}

export interface PreviewPage {
  index: number;
  thumb: string;
  width: number;
  height: number;
}

export interface PreviewSet {
  gid: number;
  previews: PreviewPage[];
  total: number;
}

export interface GalleryDetail extends GalleryInfo {
  tags: TagGroup[];
  comments: Comment[];
  /** Total comment count (may exceed `comments.length` when more pages exist). */
  commentCount: number;
  /** True when the comment section has more pages than the ones in `comments`. */
  hasMoreComments: boolean;
  previewSet: PreviewSet;
  isFavorited: boolean;
  favoriteSlot: number; // -1 = not favorited
  myRating: number;
}

export interface PageData {
  index: number;
  url: string;
  width: number;
  height: number;
}

export interface ListParams {
  page: number;
  search: string;
  categories: Category[] | null; // null = all
  minRating: number | null;
  minPages: number | null;
  maxPages: number | null;
  /** e-hentai f_sname: include gallery names in text search. */
  enableUploader: boolean;
  /** e-hentai f_stags: include gallery tags in text search. */
  enableTags: boolean;
  /** e-hentai f_sdesc: search gallery descriptions. */
  searchDesc: boolean;
  /** e-hentai f_storr: search torrent filenames. */
  searchTorrent: boolean;
  /** e-hentai f_sdt1: search low-power tags. */
  searchLowPowerTags: boolean;
  /** e-hentai f_sdt2: search downvoted tags. */
  searchDownvotedTags: boolean;
  /** e-hentai f_sh: search expunged galleries. */
  searchExpunged: boolean;
}

export const DEFAULT_LIST_PARAMS: ListParams = {
  page: 0,
  search: "",
  categories: null,
  minRating: null,
  minPages: null,
  maxPages: null,
  enableUploader: true,
  enableTags: true,
  searchDesc: false,
  searchTorrent: false,
  searchLowPowerTags: false,
  searchDownvotedTags: false,
  searchExpunged: false,
};

export type ToplistTimespan = "yesterday" | "month" | "year" | "all";

export interface FavoriteItem extends GalleryInfo {
  favoriteSlot: number;
  favoriteNote: string;
  favoritedTime: number;
}

// ---------------------------------------------------------------------------
// Downloads (`DownloadDto` in src-tauri/src/commands/downloads.rs).
// `thumb` is a regular (uncached) URL served through the image cache like
// any other gallery thumb.
// ---------------------------------------------------------------------------

export type DownloadState =
  | "none"
  | "queued"
  | "downloading"
  | "paused"
  | "done"
  | "failed";

export interface DownloadedGallery {
  gid: number;
  token: string;
  title: string;
  pages: number;
  availablePages: number[];
}

export interface DownloadEntry {
  gid: number;
  token: string;
  title: string;
  thumb: string;
  category: string;
  pages: number;
  state: DownloadState;
  label: string | null;
  dirname: string | null;
  finishedPages: number;
  localThumb?: string | null;
  dirPath: string | null;
  error: string | null;
  time: number; // epoch milliseconds
}

// ---------------------------------------------------------------------------
// Torrents and archiver (`TorrentDto` / `ArchiverInfoDto` in src-tauri).
// `TorrentInfo.size` is bytes; `ArchiverInfo.params` mirrors the archiver
// form's hidden fields.
// ---------------------------------------------------------------------------

export interface TorrentInfo {
  name: string;
  size: number; // bytes
  seeds: number;
  peers: number;
  downloads: number;
  url: string;
}

export interface ArchiverInfo {
  action: string;
  params: Array<[string, string]>;
  costText: string;
}

export type ArchiveResolution = "res" | "org";

// ---------------------------------------------------------------------------
// News (`NewsItemDto`), quota (`QuotaDto`) and search history.
// Quota counters are -1 (or null) when unknown; `imageResetSeconds` /
// `searchResetSeconds` count down to the next reset when known.
// ---------------------------------------------------------------------------

export interface NewsItem {
  title: string;
  date: string;
  body: string;
  link: string;
}

export interface Quota {
  imagesUsed: number | null;
  imagesTotal: number | null;
  imageResetSeconds: number | null;
  searchesUsed: number | null;
  searchesTotal: number | null;
  searchResetSeconds: number | null;
}

// ---------------------------------------------------------------------------
// Errors — Rust rejects with a serialized `{ kind, message }` object
// (see src-tauri/src/core/error.rs).
// ---------------------------------------------------------------------------

export type ApiErrorKind =
  | "network"
  | "httpStatus"
  | "parse"
  | "notSignedIn"
  | "image509"
  | "bandwidthExceeded"
  | "cookie"
  | "db"
  | "pageUnavailable"
  | "cancelled"
  | "eh";

// ---------------------------------------------------------------------------
// History (`HistoryEntryDto` in src-tauri/src/commands/app.rs). `time` is
// epoch milliseconds (same as the mock layer's Date.now()).
// ---------------------------------------------------------------------------

export interface HistoryEntry {
  gid: number;
  token: string;
  title: string;
  thumb: string;
  category: string;
  time: number;
  pages: number;
}

// ---------------------------------------------------------------------------
// Quick searches (`QuickSearchDto`). `-1` = unset on the numeric fields.
// ---------------------------------------------------------------------------

export interface QuickSearch {
  id: number;
  name: string;
  keyword: string;
  categories: string | null;
  minRating: number;
  minPages: number;
  maxPages: number;
  time: number;
}

// ---------------------------------------------------------------------------
// Blacklist (`BlacklistDto`).
// ---------------------------------------------------------------------------

export interface BlacklistEntry {
  id: number;
  type: string; // "title" | "uploader" | "tag" | "tag_namespace" | ...
  content: string;
  time: number;
}

// ---------------------------------------------------------------------------
// Network config — verbatim mirror of `NetworkConfig` /
// `SetNetworkConfigRequest` in src-tauri/src/app_state.rs / commands/app.rs.
// ---------------------------------------------------------------------------

export interface NetworkConfig {
  site: string; // "e" | "ex"
  proxyType: string; // "none" | "http" | "socks5"
  proxyHost: string;
  proxyPort: number;
  dohEnabled: boolean;
  hosts: Record<string, string[]>;
  imageCacheSizeMb: number;
  imageResolution: string;
  thumbResolution: number; // 0 = original, 1 = 250px, 2 = 300px
  fixThumbUrl: boolean;
}

// ---------------------------------------------------------------------------
// Tag-translation database (`TagSuggestionDto` / `TagDbStatusDto` in
// src-tauri/src/commands/tags.rs). An empty `name` marks a namespace
// suggestion (query was a bare prefix of a namespace).
// ---------------------------------------------------------------------------

export interface TagSuggestion {
  namespace: string;
  name: string;
  translated: string;
}

export interface TagDbStatus {
  available: boolean;
  /** sha1 of the packed artifact; "" when unavailable. */
  version: string;
  entryCount: number;
}
