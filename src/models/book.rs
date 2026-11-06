#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BookFormat {
    Cbz,
    Epub,
    Pdf,
}

#[allow(dead_code)]
impl BookFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            BookFormat::Cbz => "CBZ",
            BookFormat::Epub => "EPUB",
            BookFormat::Pdf => "PDF",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "cbz" | "zip" => Some(BookFormat::Cbz),
            "epub" => Some(BookFormat::Epub),
            "pdf" => Some(BookFormat::Pdf),
            _ => None,
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CBZ" | "ZIP" => Some(BookFormat::Cbz),
            "EPUB" => Some(BookFormat::Epub),
            "PDF" => Some(BookFormat::Pdf),
            _ => None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Library {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Book {
    pub id: i64,
    pub library_id: i64,
    pub title: String,
    pub series: String,
    pub volume: String,
    pub chapter: String,
    pub format: String,
    pub file_path: String,
    pub file_size: i64,
    pub page_count: i64,
    pub cover_image_path: Option<String>,
    pub summary: Option<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookChapter {
    pub id: i64,
    pub book_id: i64,
    pub chapter_index: i64,
    pub title: String,
    pub content_href: String,
    pub word_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSummaryDto {
    pub id: i64,
    pub title: String,
    pub series: String,
    pub volume: String,
    pub chapter: String,
    pub format: String,
    pub page_count: i64,
    pub current_page: i64,
    pub progress_percent: f64,
    pub is_finished: bool,
    pub has_cover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetailDto {
    pub book: Book,
    pub chapters: Vec<BookChapter>,
    pub current_page: i64,
    pub scroll_anchor: Option<String>,
    pub is_finished: bool,
    pub progress_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BookFilterQuery {
    pub query: Option<String>,
    pub format: Option<String>,
    pub library_id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultDto {
    pub status: String,
    pub scanned_files: usize,
    pub added_books: usize,
    pub updated_books: usize,
    pub removed_books: usize,
    pub elapsed_ms: u128,
}
