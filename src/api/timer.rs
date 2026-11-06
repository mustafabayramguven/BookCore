use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::queries::sessions::{
    get_reading_stats, heartbeat, pause_session, start_session, stop_session,
};
use crate::error::AppResult;
use crate::models::{
    HeartbeatRequest, HeartbeatResponse, PauseSessionRequest, ReadingStatsDto, StartSessionRequest,
    StartSessionResponse, StopSessionRequest,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub book_id: Option<i64>,
}

pub async fn start_timer_handler(
    State(state): State<AppState>,
    Json(req): Json<StartSessionRequest>,
) -> AppResult<Json<StartSessionResponse>> {
    let res = start_session(&state.pool, &req).await?;
    Ok(Json(res))
}

pub async fn heartbeat_handler(
    State(state): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> AppResult<Json<HeartbeatResponse>> {
    let res = heartbeat(
        &state.pool,
        req.session_id,
        req.current_page,
        req.delta_seconds,
    )
    .await?;
    Ok(Json(res))
}

pub async fn pause_timer_handler(
    State(state): State<AppState>,
    Json(req): Json<PauseSessionRequest>,
) -> AppResult<Json<Value>> {
    pause_session(&state.pool, req.session_id).await?;
    Ok(Json(
        json!({ "status": "paused", "session_id": req.session_id }),
    ))
}

pub async fn stop_timer_handler(
    State(state): State<AppState>,
    Json(req): Json<StopSessionRequest>,
) -> AppResult<Json<Value>> {
    stop_session(&state.pool, req.session_id, req.final_page).await?;
    Ok(Json(
        json!({ "status": "stopped", "session_id": req.session_id }),
    ))
}

pub async fn stats_handler(
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> AppResult<Json<ReadingStatsDto>> {
    let stats = get_reading_stats(&state.pool, query.book_id).await?;
    Ok(Json(stats))
}
