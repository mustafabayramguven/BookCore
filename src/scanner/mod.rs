pub mod archive;
pub mod comic_info;
pub mod epub;
pub mod pdf;
pub mod walk;

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;
use tracing::{error, info};

use crate::db::queries::books::{
    delete_book_by_id, get_all_book_paths_in_library, replace_book_chapters, upsert_book, NewBook,
    NewChapter,
};
use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::{BookFormat, ScanResultDto};

pub async fn scan_library(
    pool: &DbPool,
    library_id: i64,
    library_path: &Path,
    covers_dir: &Path,
) -> AppResult<ScanResultDto> {
    let start_time = Instant::now();
    info!(
        "Starting scan for library {} at {:?}",
        library_id, library_path
    );

    let lib_path_buf = library_path.to_path_buf();
    let discovered =
        tokio::task::spawn_blocking(move || walk::discover_books_in_directory(&lib_path_buf))
            .await
            .map_err(|e| {
                crate::error::AppError::Internal(format!("Spawn blocking error: {}", e))
            })?;

    let mut scanned_files = 0;
    let mut added_books = 0;
    let mut updated_books = 0;
    let mut discovered_paths = HashSet::new();

    for file_info in &discovered {
        scanned_files += 1;
        let file_path_str = file_info.path.to_string_lossy().to_string();
        discovered_paths.insert(file_path_str.clone());

        let path_clone = file_info.path.clone();
        let format = file_info.format;
        let file_size = file_info.file_size;

        match format {
            BookFormat::Cbz => {
                let parse_res = tokio::task::spawn_blocking(move || {
                    let pages = archive::list_archive_pages(&path_clone).unwrap_or_default();
                    let page_count = pages.len() as i64;
                    let comic_info = archive::extract_comic_info(&path_clone).unwrap_or_default();
                    let cover_data = archive::extract_cover_image(&path_clone).unwrap_or_default();
                    let file_name = path_clone
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let (h_title, h_series, h_vol, h_ch) =
                        archive::parse_filename_heuristics(file_name);
                    (
                        page_count, comic_info, cover_data, h_title, h_series, h_vol, h_ch,
                    )
                })
                .await;

                if let Ok((page_count, comic_info, cover_data, h_title, h_series, h_vol, h_ch)) =
                    parse_res
                {
                    let (title, series, volume, chapter, summary, genre, language) =
                        if let Some(ci) = comic_info {
                            (
                                ci.title.unwrap_or(h_title),
                                ci.series.unwrap_or(h_series),
                                ci.volume.unwrap_or(h_vol),
                                ci.number.unwrap_or(h_ch),
                                ci.summary,
                                ci.genre,
                                ci.language_iso,
                            )
                        } else {
                            (h_title, h_series, h_vol, h_ch, None, None, None)
                        };

                    let new_book = NewBook {
                        library_id,
                        title,
                        series,
                        volume,
                        chapter,
                        format: "CBZ".to_string(),
                        file_path: file_path_str.clone(),
                        file_size,
                        page_count,
                        cover_image_path: None,
                        summary,
                        genre,
                        language,
                    };

                    match upsert_book(pool, &new_book).await {
                        Ok((book_id, is_new)) => {
                            if is_new {
                                added_books += 1;
                            } else {
                                updated_books += 1;
                            }

                            // Save extracted cover
                            if let Some((cover_name, cover_bytes)) = cover_data {
                                let ext = Path::new(&cover_name)
                                    .extension()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("jpg");
                                let cover_filename = format!("book_{}.{}", book_id, ext);
                                let cover_dest = covers_dir.join(&cover_filename);
                                if std::fs::write(&cover_dest, &cover_bytes).is_ok() {
                                    let cover_path_str = cover_dest.to_string_lossy().to_string();
                                    let _ = sqlx::query(
                                        "UPDATE books SET cover_image_path = ? WHERE id = ?",
                                    )
                                    .bind(cover_path_str)
                                    .bind(book_id)
                                    .execute(pool)
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to upsert CBZ book {}: {}", file_path_str, e);
                        }
                    }
                }
            }
            BookFormat::Epub => {
                let parse_res =
                    tokio::task::spawn_blocking(move || epub::parse_epub(&path_clone)).await;

                if let Ok(Ok(meta)) = parse_res {
                    let page_count = meta.spine_items.len().max(1) as i64;
                    let file_name = file_info
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let (_, h_series, h_vol, h_ch) = archive::parse_filename_heuristics(file_name);

                    let series = meta.series.unwrap_or(h_series);
                    let volume = meta.volume.unwrap_or(h_vol);

                    let new_book = NewBook {
                        library_id,
                        title: meta.title,
                        series,
                        volume,
                        chapter: h_ch,
                        format: "EPUB".to_string(),
                        file_path: file_path_str.clone(),
                        file_size,
                        page_count,
                        cover_image_path: None,
                        summary: meta.summary,
                        genre: None,
                        language: meta.language,
                    };

                    match upsert_book(pool, &new_book).await {
                        Ok((book_id, is_new)) => {
                            if is_new {
                                added_books += 1;
                            } else {
                                updated_books += 1;
                            }

                            // Save EPUB chapters
                            let chapters: Vec<NewChapter> = meta
                                .spine_items
                                .iter()
                                .enumerate()
                                .map(|(idx, item)| NewChapter {
                                    chapter_index: idx as i64,
                                    title: item.title.clone(),
                                    content_href: item.full_path.clone(),
                                    word_count: 0,
                                })
                                .collect();

                            let _ = replace_book_chapters(pool, book_id, &chapters).await;

                            // Extract cover if found
                            if let Some(cover_entry) = meta.cover_image_path {
                                let ext = Path::new(&cover_entry)
                                    .extension()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("jpg")
                                    .to_string();
                                let cover_entry_clone = cover_entry.clone();
                                let epub_path_clone = file_info.path.clone();
                                if let Ok(cover_bytes) = tokio::task::spawn_blocking(move || {
                                    epub::extract_epub_entry(&epub_path_clone, &cover_entry_clone)
                                })
                                .await
                                .unwrap_or_else(|_| {
                                    Err(crate::error::AppError::NotFound(
                                        "Cover not found".to_string(),
                                    ))
                                }) {
                                    let cover_filename = format!("book_{}.{}", book_id, ext);
                                    let cover_dest = covers_dir.join(&cover_filename);
                                    if std::fs::write(&cover_dest, &cover_bytes).is_ok() {
                                        let cover_path_str =
                                            cover_dest.to_string_lossy().to_string();
                                        let _ = sqlx::query(
                                            "UPDATE books SET cover_image_path = ? WHERE id = ?",
                                        )
                                        .bind(cover_path_str)
                                        .bind(book_id)
                                        .execute(pool)
                                        .await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to upsert EPUB book {}: {}", file_path_str, e);
                        }
                    }
                }
            }
            BookFormat::Pdf => {
                let parse_res =
                    tokio::task::spawn_blocking(move || pdf::parse_pdf(&path_clone)).await;

                if let Ok(Ok(meta)) = parse_res {
                    let file_name = file_info
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let (_, h_series, h_vol, h_ch) = archive::parse_filename_heuristics(file_name);

                    let new_book = NewBook {
                        library_id,
                        title: meta.title,
                        series: h_series,
                        volume: h_vol,
                        chapter: h_ch,
                        format: "PDF".to_string(),
                        file_path: file_path_str.clone(),
                        file_size,
                        page_count: meta.page_count,
                        cover_image_path: None,
                        summary: None,
                        genre: None,
                        language: None,
                    };

                    match upsert_book(pool, &new_book).await {
                        Ok((_, is_new)) => {
                            if is_new {
                                added_books += 1;
                            } else {
                                updated_books += 1;
                            }
                        }
                        Err(e) => {
                            error!("Failed to upsert PDF book {}: {}", file_path_str, e);
                        }
                    }
                }
            }
        }
    }

    // Tombstone removal: delete books from DB that no longer exist on disk
    let existing_db_books = get_all_book_paths_in_library(pool, library_id).await?;
    let mut removed_books = 0;
    for (b_id, b_path) in existing_db_books {
        if !discovered_paths.contains(&b_path) {
            if delete_book_by_id(pool, b_id).await.is_ok() {
                removed_books += 1;
            }
        }
    }

    let elapsed_ms = start_time.elapsed().as_millis();
    info!(
        "Scan completed in {}ms: {} scanned, {} added, {} updated, {} removed",
        elapsed_ms, scanned_files, added_books, updated_books, removed_books
    );

    Ok(ScanResultDto {
        status: "completed".to_string(),
        scanned_files,
        added_books,
        updated_books,
        removed_books,
        elapsed_ms,
    })
}
