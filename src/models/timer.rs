#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingSession {
    pub id: i64,
    pub book_id: i64,
    pub started_at: String,
    pub last_heartbeat_at: String,
    pub ended_at: Option<String>,
    pub active_seconds: i64,
    pub start_page: i64,
    pub end_page: i64,
    pub pages_read: i64,
    pub is_pomodoro: bool,
    pub goal_minutes: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DailyStats {
    pub date_str: String,
    pub total_seconds: i64,
    pub pages_read: i64,
    pub books_touched: i64,
    pub sessions_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartSessionRequest {
    pub book_id: i64,
    pub current_page: Option<i64>,
    pub is_pomodoro: Option<bool>,
    pub goal_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartSessionResponse {
    pub session_id: i64,
    pub started_at: String,
    pub goal_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatRequest {
    pub session_id: i64,
    pub current_page: i64,
    pub delta_seconds: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatResponse {
    pub status: String,
    pub active_seconds: i64,
    pub pages_read: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PauseSessionRequest {
    pub session_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopSessionRequest {
    pub session_id: i64,
    pub final_page: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayStatsDto {
    pub seconds: i64,
    pub minutes: i64,
    pub pages: i64,
    pub goal_minutes: i64,
    pub goal_achieved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentDayStatDto {
    pub date: String,
    pub minutes: i64,
    pub pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentSessionDto {
    pub id: i64,
    pub book_id: i64,
    pub book_title: String,
    pub date: String,
    pub active_minutes: i64,
    pub pages_read: i64,
    pub ppm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingStatsDto {
    pub current_streak_days: i64,
    pub longest_streak_days: i64,
    pub total_reading_seconds: i64,
    pub total_reading_hours: f64,
    pub total_pages_read: i64,
    pub pages_per_minute: f64,
    pub today: TodayStatsDto,
    pub recent_days: Vec<RecentDayStatDto>,
    pub recent_sessions: Vec<RecentSessionDto>,
}
