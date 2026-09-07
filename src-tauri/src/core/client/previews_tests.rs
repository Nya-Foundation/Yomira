use super::super::ClientConfig;
use super::*;
use crate::core::models::PreviewItem;

fn preview_item(position: i32, token: &str) -> PreviewItem {
    PreviewItem {
        position,
        url: String::new(),
        gid: 42,
        token: token.to_string(),
        xres: 0,
        yres: 0,
        xoffset: 0,
        yoffset: 0,
    }
}

/// Client whose proxy points at a closed port: any accidental network
/// call fails fast instead of hanging the test suite.
fn offline_client() -> EhClient {
    EhClient::with_defaults(ClientConfig {
        proxy: Some("http://127.0.0.1:1".to_string()),
        ..Default::default()
    })
    .unwrap()
}

#[tokio::test]
async fn resolve_ptoken_serves_neighbours_from_harvested_cache() {
    let client = offline_client();
    // Seed the cache the way fetch_preview_set_shared would: one preview
    // page carrying ptokens for pages 4 and 5.
    let mut set = PreviewSet::new(0);
    set.rows.push(preview_item(4, "pt-4"));
    set.rows.push(preview_item(5, "pt-5"));
    {
        let mut cache = client.preview_cache.lock().await;
        let set = Arc::new(set);
        harvest_ptokens(&mut cache, 42, "gt", &set);
        cache.sets.insert((42, "gt".to_string(), 0), set);
    }
    // Both pages resolve straight from the harvested map — no network.
    assert_eq!(client.resolve_ptoken(42, "gt", 4).await.unwrap(), "pt-4");
    assert_eq!(client.resolve_ptoken(42, "gt", 5).await.unwrap(), "pt-5");
}

#[tokio::test]
async fn resolve_ptoken_harvests_preexisting_sets_before_fetching() {
    let client = offline_client();
    // A set cached by an earlier (e.g. download-engine) call, never
    // harvested: resolve_ptoken must scan + harvest it without fetching.
    let mut set = PreviewSet::new(0);
    set.rows.push(preview_item(0, "pt-0"));
    {
        let mut cache = client.preview_cache.lock().await;
        cache.sets.insert((7, "g7".to_string(), 0), Arc::new(set));
    }
    assert_eq!(client.resolve_ptoken(7, "g7", 0).await.unwrap(), "pt-0");
}

#[tokio::test]
async fn resolve_ptoken_falls_back_to_api_when_covered_without_token() {
    let client = offline_client();
    // A set that covers page 3 but carries an empty ptoken for it must
    // short-circuit to the (here unreachable) API fallback instead of
    // scanning or returning the empty token.
    let mut set = PreviewSet::new(0);
    set.rows.push(preview_item(3, ""));
    {
        let mut cache = client.preview_cache.lock().await;
        cache.sets.insert((9, "g9".to_string(), 0), Arc::new(set));
    }
    assert!(client.resolve_ptoken(9, "g9", 3).await.is_err());
}
