//! Tag-translation database commands: autocomplete suggestions and status.

use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;
use crate::core::error::EhError;
use crate::tags::{namespace_to_prefix, Suggestion};

/// `types.ts` `TagSuggestion`. An empty `name` marks a namespace suggestion.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionDto {
    pub namespace: String,
    pub name: String,
    pub translated: String,
}

/// `types.ts` `TagDbStatus`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDbStatusDto {
    pub available: bool,
    /// sha1 of the packed artifact the in-memory DB was verified against.
    pub version: String,
    pub entry_count: usize,
}

fn suggestion_dto(s: Suggestion) -> TagSuggestionDto {
    TagSuggestionDto {
        namespace: s.namespace,
        name: s.name,
        translated: s.translated,
    }
}

#[tauri::command]
pub async fn suggest_tags(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<TagSuggestionDto>, EhError> {
    // `suggest` linear-scans ~200k entries; run it on the blocking thread
    // pool so per-keystroke calls never occupy an async runtime worker.
    let db = state.tag_db.inner();
    let hints = tauri::async_runtime::spawn_blocking(move || {
        db.blocking_read()
            .as_ref()
            .map(|d| d.suggest(&query))
            .unwrap_or_default()
    })
    .await
    .map_err(|e| EhError::Eh(format!("tag suggest task failed: {e}")))?;
    Ok(hints.into_iter().map(suggestion_dto).collect())
}

#[tauri::command]
pub async fn tag_db_status(state: State<'_, AppState>) -> Result<TagDbStatusDto, EhError> {
    let (available, version, entry_count) = state.tag_db.status().await;
    Ok(TagDbStatusDto {
        available,
        version,
        entry_count,
    })
}

#[tauri::command]
pub async fn update_tag_db(state: State<'_, AppState>) -> Result<(), EhError> {
    state
        .tag_db
        .update(true)
        .await
        .map_err(|e| EhError::Eh(format!("tag database update failed: {e}")))
}

/// `types.ts` `TagTranslateRequest` input.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagTranslateRequest {
    pub namespace: String,
    pub name: String,
}

/// Build the packed-DB lookup key for one tag, EhViewer convention:
/// `prefix + ":" + name` with the detail page's display-name spaces turned
/// into underscores (`f:big_breasts`); misc tags are bare names. Unknown
/// namespaces have no prefix and yield `None`.
fn build_translate_key(namespace: &str, name: &str) -> Option<String> {
    let prefix = namespace_to_prefix(namespace)?;
    Some(format!("{}{}", prefix, name.replace(' ', "_")))
}

#[tauri::command]
pub async fn translate_tags(
    state: State<'_, AppState>,
    tags: Vec<TagTranslateRequest>,
) -> Result<Vec<Option<String>>, EhError> {
    // Build every lookup key first, then one read guard for the whole batch
    // (detail pages send 50-150 tags).
    let key_of: Vec<Option<String>> = tags
        .iter()
        .map(|t| build_translate_key(&t.namespace, &t.name))
        .collect();
    let flat: Vec<String> = key_of.iter().flatten().cloned().collect();
    let translated = state.tag_db.translate_many(&flat).await;
    let mut hits = translated.into_iter();
    Ok(key_of
        .into_iter()
        .map(|key| key.and_then(|_| hits.next().unwrap_or(None)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_lookup_keys_per_ehviewer_convention() {
        assert_eq!(
            build_translate_key("female", "big breasts").as_deref(),
            Some("f:big_breasts")
        );
        assert_eq!(
            build_translate_key("misc", "full color").as_deref(),
            Some("full_color")
        );
        // Namespaces with multi-letter prefixes keep them.
        assert_eq!(
            build_translate_key("cosplayer", "some one").as_deref(),
            Some("cos:some_one")
        );
        // Unknown namespace → no lookup.
        assert_eq!(build_translate_key("bogus", "tag"), None);
        // Underscore-only names pass through unchanged.
        assert_eq!(
            build_translate_key("female", "big_breasts").as_deref(),
            Some("f:big_breasts")
        );
    }
}
