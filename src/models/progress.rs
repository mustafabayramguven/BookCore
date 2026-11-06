#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingProgress {
    pub id: i64,
    pub book_id: i64,
    pub current_page: i64,
    pub scroll_anchor: Option<String>,
    pub is_finished: bool,
    pub total_reads: i64,
    pub last_read_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdateDto {
    pub page: i64,
    pub scroll_anchor: Option<String>,
    pub is_finished: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bookmark {
    pub id: i64,
    pub book_id: i64,
    pub page_number: i64,
    pub title: Option<String>,
    pub xpath: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookmarkDto {
    pub page_number: i64,
    pub title: Option<String>,
    pub xpath: Option<String>,
}
