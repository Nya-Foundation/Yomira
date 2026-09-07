-- Exviewer search history (search-term suggestions).
--
-- One row per distinct search term; `time` is the unix epoch milliseconds
-- of the most recent search, refreshed on every repeat (upsert). The term
-- itself is the primary key so re-searching just moves it to the top.
-- Registered with both the rusqlite `db` module and tauri-plugin-sql,
-- exactly like migrations/0001_init.sql.

CREATE TABLE IF NOT EXISTS search_history (
    term TEXT PRIMARY KEY NOT NULL,
    time INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS index_search_history_time ON search_history (time);
