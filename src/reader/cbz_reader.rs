#![allow(dead_code)]

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::scanner::archive::{list_archive_pages, read_archive_entry};

pub fn get_cbz_page(archive_path: &Path, page_num: usize) -> AppResult<(Vec<u8>, String)> {
    let pages = list_archive_pages(archive_path)?;
    if pages.is_empty() {
        return Err(AppError::NotFound(
            "Archive has no readable image pages".to_string(),
        ));
    }

    if page_num >= pages.len() {
        return Err(AppError::NotFound(format!(
            "Page {} out of range (total pages: {})",
            page_num,
            pages.len()
        )));
    }

    let page_entry = &pages[page_num];
    let bytes = read_archive_entry(archive_path, page_entry)?;

    let mime = mime_guess::from_path(page_entry)
        .first_or_octet_stream()
        .to_string();

    Ok((bytes, mime))
}
