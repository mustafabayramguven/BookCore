#![allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::type_complexity,
    dead_code
)]

mod api;
mod config;
mod db;
mod error;
mod models;
mod reader;
mod scanner;
mod state;
mod timer;
mod web;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::AppConfig;
use crate::db::init_db;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bookcore=info,tower_http=info".into()),
        )
        .init();

    info!("Starting BookCore v0.1.0...");

    let config = AppConfig::from_env();
    info!(
        "Configuration: Port={}, DB Path={:?}, Books Dir={:?}, Cache Dir={:?}",
        config.port, config.db_path, config.books_dir, config.cache_dir
    );

    let pool = init_db(&config).await?;
    info!("Database initialized successfully");

    let state = AppState::new(config.clone(), pool);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", api::api_router())
        .merge(web::web_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("BookCore server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("BookCore server shut down cleanly");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C signal handler");
    info!("Shutdown signal received, shutting down gracefully...");
}
