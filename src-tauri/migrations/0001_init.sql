-- Exviewer initial schema.
--
-- Ported from the original greenDAO dao package
-- (github.com/xiaojieonly/Ehviewer_CN_SXJ, branch BiLi_PC_Gamer):
--   DownloadsDao / DownloadLabelDao  -> downloads, download_labels
--   HistoryDao                       -> history
--   LocalFavoritesDao                -> local_favorites
--   QuickSearchDao                   -> quick_search
--   BlackListDao                     -> blacklist
--   FilterDao                        -> filter
-- plus read positions and a generic settings key/value store.
--
-- This file is the single source of truth: it is embedded by the Rust
-- `db` module (rusqlite) and registered with tauri-plugin-sql so the
-- frontend can query the same tables. A unit test asserts the two stay
-- identical. Times are unix epoch milliseconds unless noted (the original
-- uses System.currentTimeMillis).

CREATE TABLE IF NOT EXISTS history (
    gid      INTEGER PRIMARY KEY NOT NULL,
    token    TEXT,
    title    TEXT,
    thumb    TEXT,
    category INTEGER NOT NULL DEFAULT 0,
    time     INTEGER NOT NULL DEFAULT 0,
    pages    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS local_favorites (
    gid      INTEGER PRIMARY KEY NOT NULL,
    token    TEXT,
    title    TEXT,
    thumb    TEXT,
    category INTEGER NOT NULL DEFAULT 0,
    time     INTEGER NOT NULL DEFAULT 0,
    pages    INTEGER NOT NULL DEFAULT 0,
    slot     INTEGER NOT NULL DEFAULT -1
);

CREATE TABLE IF NOT EXISTS quick_search (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL DEFAULT '',
    keyword    TEXT,
    categories TEXT,
    min_rating INTEGER NOT NULL DEFAULT -1,
    min_pages  INTEGER NOT NULL DEFAULT -1,
    max_pages  INTEGER NOT NULL DEFAULT -1,
    time       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS blacklist (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    type    TEXT NOT NULL DEFAULT 'title',
    content TEXT NOT NULL DEFAULT '',
    time    INTEGER NOT NULL DEFAULT 0
);

-- EhFilter entries: `mode` is one of title / uploader / tag / tag_namespace.
CREATE TABLE IF NOT EXISTS filter (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL DEFAULT 'title',
    text TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS read_position (
    gid  INTEGER PRIMARY KEY NOT NULL,
    page INTEGER NOT NULL DEFAULT 0,
    time INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings_kv (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT
);

-- Reserved for phase 2 (downloads); defined now so no later migration
-- rewrite is needed. `legacy` holds the original JSON-encoded extras.
CREATE TABLE IF NOT EXISTS downloads (
    gid         INTEGER PRIMARY KEY NOT NULL,
    token       TEXT,
    title       TEXT,
    title_jpn   TEXT,
    thumb       TEXT,
    category    INTEGER NOT NULL DEFAULT 0,
    posted      TEXT,
    uploader    TEXT,
    rating      REAL NOT NULL DEFAULT -1,
    language    TEXT,
    state       INTEGER NOT NULL DEFAULT 0,
    legacy      INTEGER NOT NULL DEFAULT 0,
    time        INTEGER NOT NULL DEFAULT 0,
    label       TEXT,
    dirname     TEXT,
    archive_uri TEXT
);

CREATE TABLE IF NOT EXISTS download_labels (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL DEFAULT '',
    time INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS index_history_time ON history (time);
CREATE INDEX IF NOT EXISTS index_local_favorites_time ON local_favorites (time);
CREATE INDEX IF NOT EXISTS index_quick_search_time ON quick_search (time);
