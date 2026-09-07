//! Image serving, local library data and network settings commands.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::{AppState, NetworkConfig};
use crate::core::error::EhError;
use crate::core::models::GalleryInfo;
use crate::db::{BlacklistEntry, HistoryEntry, QuickSearchEntry};
use crate::image_cache;

#[tauri::command]
pub async fn get_image(state: State<'_, AppState>, url: String) -> Result<String, EhError> {
    image_cache::get_image(
        &state.client,
        &state.image_cache,
        state.image_server_port,
        &url,
    )
    .await
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// `HistoryEntry` in the camelCase wire shape.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryDto {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub thumb: String,
    pub category: String,
    pub time: i64,
    pub pages: i32,
}

fn history_entry_dto(e: &HistoryEntry) -> HistoryEntryDto {
    HistoryEntryDto {
        gid: e.gid,
        token: e.token.clone(),
        title: e.title.clone(),
        thumb: e.thumb.clone(),
        category: crate::core::models::category::category_name(e.category).to_string(),
        time: e.time,
        pages: e.pages,
    }
}

#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    page: i64,
) -> Result<Vec<HistoryEntryDto>, EhError> {
    let db = Arc::clone(&state.db);
    let entries = tokio::task::spawn_blocking(move || db.history(page, 50)).await??;
    Ok(entries.iter().map(history_entry_dto).collect())
}

/// Explicit history insert (the gallery-detail command already records
/// implicitly; this endpoint supports explicit history insertion).
#[tauri::command]
pub async fn record_history(
    state: State<'_, AppState>,
    gid: u64,
    token: String,
    title: String,
    thumb: String,
    category: i32,
    pages: i32,
) -> Result<(), EhError> {
    let info = GalleryInfo {
        gid,
        token,
        title,
        thumb,
        category,
        pages,
        ..Default::default()
    };
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.record_history(&info)).await??;
    Ok(())
}

#[tauri::command]
pub async fn delete_history(state: State<'_, AppState>, gid: u64) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.delete_history(gid as i64)).await??;
    Ok(())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.clear_history()).await??;
    Ok(())
}

// ---------------------------------------------------------------------------
// Quick search CRUD
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickSearchDto {
    pub id: i64,
    pub name: String,
    pub keyword: String,
    pub categories: Option<String>,
    pub min_rating: i32,
    pub min_pages: i32,
    pub max_pages: i32,
    pub time: i64,
}

fn quick_search_dto(e: &QuickSearchEntry) -> QuickSearchDto {
    QuickSearchDto {
        id: e.id,
        name: e.name.clone(),
        keyword: e.keyword.clone(),
        categories: e.categories.clone(),
        min_rating: e.min_rating,
        min_pages: e.min_pages,
        max_pages: e.max_pages,
        time: e.time,
    }
}

#[tauri::command]
pub async fn get_quick_searches(
    state: State<'_, AppState>,
) -> Result<Vec<QuickSearchDto>, EhError> {
    let db = Arc::clone(&state.db);
    let entries = tokio::task::spawn_blocking(move || db.quick_searches()).await??;
    Ok(entries.iter().map(quick_search_dto).collect())
}

#[tauri::command]
pub async fn add_quick_search(
    state: State<'_, AppState>,
    name: String,
    keyword: String,
    categories: Option<String>,
    min_rating: Option<i32>,
    min_pages: Option<i32>,
    max_pages: Option<i32>,
) -> Result<QuickSearchDto, EhError> {
    let db = Arc::clone(&state.db);
    let name_c = name.clone();
    let keyword_c = keyword.clone();
    let categories_c = categories.clone();
    let id = tokio::task::spawn_blocking(move || {
        db.add_quick_search(
            &name_c,
            &keyword_c,
            categories_c.as_deref(),
            min_rating.unwrap_or(-1),
            min_pages.unwrap_or(-1),
            max_pages.unwrap_or(-1),
        )
    })
    .await??;
    Ok(QuickSearchDto {
        id,
        name,
        keyword,
        categories,
        min_rating: min_rating.unwrap_or(-1),
        min_pages: min_pages.unwrap_or(-1),
        max_pages: max_pages.unwrap_or(-1),
        time: chrono::Utc::now().timestamp_millis(),
    })
}

#[tauri::command]
pub async fn delete_quick_search(state: State<'_, AppState>, id: i64) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.delete_quick_search(id)).await??;
    Ok(())
}

// ---------------------------------------------------------------------------
// Blacklist CRUD
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistDto {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub content: String,
    pub time: i64,
}

fn blacklist_dto(e: &BlacklistEntry) -> BlacklistDto {
    BlacklistDto {
        id: e.id,
        kind: e.kind.clone(),
        content: e.content.clone(),
        time: e.time,
    }
}

#[tauri::command]
pub async fn get_blacklist(state: State<'_, AppState>) -> Result<Vec<BlacklistDto>, EhError> {
    let db = Arc::clone(&state.db);
    let entries = tokio::task::spawn_blocking(move || db.blacklist()).await??;
    Ok(entries.iter().map(blacklist_dto).collect())
}

#[tauri::command]
pub async fn add_blacklist(
    state: State<'_, AppState>,
    kind: String,
    content: String,
) -> Result<BlacklistDto, EhError> {
    let db = Arc::clone(&state.db);
    let kind_c = kind.clone();
    let content_c = content.clone();
    let id = tokio::task::spawn_blocking(move || db.add_blacklist(&kind_c, &content_c)).await??;
    Ok(BlacklistDto {
        id,
        kind,
        content,
        time: chrono::Utc::now().timestamp_millis(),
    })
}

#[tauri::command]
pub async fn delete_blacklist(state: State<'_, AppState>, id: i64) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.delete_blacklist(id)).await??;
    Ok(())
}

// ---------------------------------------------------------------------------
// Filters CRUD (EhFilter entries applied to list results)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterDto {
    pub id: i64,
    pub mode: String,
    pub text: String,
}

#[tauri::command]
pub async fn get_filters(state: State<'_, AppState>) -> Result<Vec<FilterDto>, EhError> {
    let db = Arc::clone(&state.db);
    let rows = tokio::task::spawn_blocking(move || db.filter_table()).await??;
    Ok(rows
        .into_iter()
        .map(|(id, mode, text)| FilterDto { id, mode, text })
        .collect())
}

#[tauri::command]
pub async fn add_filter(
    state: State<'_, AppState>,
    mode: String,
    text: String,
) -> Result<FilterDto, EhError> {
    let db = Arc::clone(&state.db);
    let mode_c = mode.clone();
    let text_c = text.clone();
    let id = tokio::task::spawn_blocking(move || db.add_filter(&mode_c, &text_c)).await??;
    Ok(FilterDto { id, mode, text })
}

#[tauri::command]
pub async fn delete_filter(state: State<'_, AppState>, id: i64) -> Result<(), EhError> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || db.delete_filter(id)).await??;
    Ok(())
}

// ---------------------------------------------------------------------------
// Network settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_network_config(state: State<'_, AppState>) -> Result<NetworkConfig, EhError> {
    NetworkConfig::load(&state.db)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkConfigRequest {
    pub site: String,
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub doh_enabled: bool,
    pub hosts: std::collections::HashMap<String, Vec<String>>,
    pub image_cache_size_mb: u64,
    pub thumb_resolution: i32,
    pub image_resolution: String,
    pub fix_thumb_url: bool,
}

#[tauri::command]
pub async fn set_network_config(
    state: State<'_, AppState>,
    config: SetNetworkConfigRequest,
) -> Result<NetworkConfig, EhError> {
    let config = NetworkConfig {
        site: config.site,
        proxy_type: config.proxy_type,
        proxy_host: config.proxy_host,
        proxy_port: config.proxy_port,
        doh_enabled: config.doh_enabled,
        hosts: config.hosts,
        image_cache_size_mb: config.image_cache_size_mb,
        thumb_resolution: config.thumb_resolution,
        image_resolution: config.image_resolution,
        fix_thumb_url: config.fix_thumb_url,
    };
    if !matches!(config.site.as_str(), "e" | "ex") {
        return Err(EhError::Eh(format!("unknown site: {}", config.site)));
    }
    if !matches!(config.proxy_type.as_str(), "none" | "http" | "socks5") {
        return Err(EhError::Eh(format!(
            "unknown proxy type: {}",
            config.proxy_type
        )));
    }
    if !matches!(config.thumb_resolution, 0..=2) {
        return Err(EhError::Eh(format!(
            "invalid thumb resolution: {}",
            config.thumb_resolution
        )));
    }

    if !matches!(
        config.image_resolution.as_str(),
        "a" | "780" | "1280" | "1600" | "2400"
    ) {
        return Err(EhError::Eh("unsupported image resolution".into()));
    }
    for (host, ips) in &config.hosts {
        if host.is_empty()
            || !host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
            || ips.is_empty()
            || ips.iter().any(|ip| ip.parse::<std::net::IpAddr>().is_err())
        {
            return Err(EhError::Eh(format!("invalid hosts entry: {host}")));
        }
    }

    // Persist, then apply to the live client (this rebuilds the reqwest
    // client when site/proxy changed; cookies survive).
    config.save(&state.db)?;
    let client_config = config.to_client_config()?;
    // The download engine owns a second client sharing our cookies/hosts;
    // keep its config in sync (site/proxy changes apply without restart).
    state.downloads.apply_client_config(&client_config)?;
    state.client.update_config(client_config)?;
    *state.hosts.write().map_err(|_| EhError::Cancelled)? = config.parsed_hosts();
    state
        .image_cache
        .set_max_bytes(config.image_cache_size_mb.saturating_mul(1024 * 1024))?;
    Ok(config)
}

#[tauri::command]
pub fn get_builtin_hosts() -> String {
    crate::core::client::dns::BUILT_IN_HOSTS_TEXT.to_string()
}
