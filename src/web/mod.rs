use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::state::AppState;

const INDEX_HTML: &str = include_str!("../ui/index.html");

pub async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

pub fn web_router() -> Router<AppState> {
    Router::new().route("/", get(index_handler))
}
