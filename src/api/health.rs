use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::db::queries::books::get_storage_stats;
use crate::error::AppResult;
use crate::state::AppState;

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": "BookCore",
        "version": "0.1.0",
        "port": 3435
    }))
}

pub async fn server_info(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let (total_books, total_libraries) = get_storage_stats(&state.pool).await?;

    Ok(Json(json!({
        "name": "BookCore",
        "version": "0.1.0",
        "db_path": state.config.db_path.to_string_lossy(),
        "storage": {
            "total_books": total_books,
            "total_libraries": total_libraries
        }
    })))
}
