#![allow(dead_code)]

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::scanner::epub::{extract_epub_entry, rewrite_epub_chapter_html};

pub fn get_epub_chapter_html(
    epub_path: &Path,
    chapter_content_href: &str,
    book_id: i64,
) -> AppResult<String> {
    let raw_bytes = extract_epub_entry(epub_path, chapter_content_href)?;
    let content_str = String::from_utf8(raw_bytes)
        .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in EPUB chapter: {}", e)))?;

    let rewritten = rewrite_epub_chapter_html(&content_str, chapter_content_href, book_id);
    Ok(rewritten)
}

pub fn get_epub_asset(epub_path: &Path, asset_file_path: &str) -> AppResult<(Vec<u8>, String)> {
    let bytes = extract_epub_entry(epub_path, asset_file_path)?;
    let mime = mime_guess::from_path(asset_file_path)
        .first_or_octet_stream()
        .to_string();

    Ok((bytes, mime))
}
