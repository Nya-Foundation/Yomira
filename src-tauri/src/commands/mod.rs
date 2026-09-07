//! Tauri command modules. The command fns themselves live in the submodules;
//! `crate::commands::all()` is the single list wired in `lib.rs`.

pub mod app;
pub mod auth;
pub mod download;
pub mod dto;
pub mod extras;
pub mod gallery;
pub mod tags;
pub mod torrent;

/// Every command exposed to the frontend (names are snake_case, matching
/// `src/lib/api.ts` exactly).
pub fn all() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        gallery::get_gallery_list,
        gallery::get_gallery_detail,
        gallery::get_preview_set,
        gallery::get_page_data,
        gallery::get_comments,
        gallery::get_favorites,
        gallery::toggle_favorite,
        gallery::rate_gallery,
        gallery::vote_comment,
        gallery::send_comment,
        torrent::get_torrent_list,
        torrent::download_torrent,
        torrent::get_archiver_info,
        torrent::download_archive,
        auth::check_sign_in,
        auth::sign_in_with_cookies,
        auth::sign_in_with_webview,
        auth::sign_out,
        auth::__cookie_probe,
        app::get_image,
        app::get_history,
        app::record_history,
        app::delete_history,
        app::clear_history,
        app::get_quick_searches,
        app::add_quick_search,
        app::delete_quick_search,
        app::get_blacklist,
        app::add_blacklist,
        app::delete_blacklist,
        app::get_filters,
        app::add_filter,
        app::delete_filter,
        app::get_network_config,
        app::get_builtin_hosts,
        app::set_network_config,
        tags::suggest_tags,
        tags::tag_db_status,
        tags::update_tag_db,
        tags::translate_tags,
        download::get_downloaded_gallery,
        download::get_downloaded_page,
        download::list_downloads,
        download::add_download,
        download::remove_download,
        download::start_download,
        download::pause_download,
        download::download_labels,
        download::add_download_label,
        download::delete_download_label,
        download::set_download_label,
        download::get_download_root,
        download::set_download_root,
        extras::image_lookup,
        extras::get_news,
        extras::get_quota,
        extras::modify_favorite_note,
        extras::search_history,
        extras::record_search,
        extras::clear_search_history,
        extras::delete_search_term,
    ]
}
