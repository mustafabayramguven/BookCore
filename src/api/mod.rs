pub mod bookmarks;
pub mod books;
pub mod health;
pub mod progress;
pub mod reader;
pub mod timer;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        // Health & Info
        .route("/health", get(health::health_check))
        .route("/info", get(health::server_info))
        // Scanner & Books
        .route("/scan", post(books::scan_handler))
        .route("/books", get(books::list_books_handler))
        .route(
            "/books/:id",
            get(books::get_book_handler).delete(books::delete_book_handler),
        )
        // Reader content
        .route("/books/:id/cover", get(reader::get_cover_handler))
        .route("/books/:id/pages/:page_num", get(reader::get_page_handler))
        .route("/books/:id/chapters", get(reader::get_chapters_handler))
        .route(
            "/books/:id/chapters/:chapter_idx",
            get(reader::get_chapter_content_handler),
        )
        .route("/books/:id/assets", get(reader::get_asset_handler))
        .route("/books/:id/raw", get(reader::get_raw_pdf_handler))
        // Progress & Bookmarks
        .route(
            "/books/:id/progress",
            get(progress::get_progress_handler).post(progress::save_progress_handler),
        )
        .route(
            "/books/:id/bookmarks",
            get(bookmarks::list_bookmarks_handler).post(bookmarks::create_bookmark_handler),
        )
        .route("/bookmarks/:id", delete(bookmarks::delete_bookmark_handler))
        // Timer & Tracker
        .route("/timer/start", post(timer::start_timer_handler))
        .route("/timer/heartbeat", post(timer::heartbeat_handler))
        .route("/timer/pause", post(timer::pause_timer_handler))
        .route("/timer/stop", post(timer::stop_timer_handler))
        .route("/timer/stats", get(timer::stats_handler))
}
