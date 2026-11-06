#![allow(dead_code)]

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{Book, BookChapter, BookFilterQuery, BookSummaryDto, Library};

#[derive(Debug, Clone)]
pub struct NewBook {
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
}

#[derive(Debug, Clone)]
pub struct NewChapter {
    pub chapter_index: i64,
    pub title: String,
    pub content_href: String,
    pub word_count: i64,
}

pub async fn list_libraries(pool: &DbPool) -> AppResult<Vec<Library>> {
    let libs = sqlx::query_as::<_, Library>(
        "SELECT id, name, path, created_at, updated_at FROM libraries ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(libs)
}

pub async fn get_library_by_id(pool: &DbPool, id: i64) -> AppResult<Option<Library>> {
    let lib = sqlx::query_as::<_, Library>(
        "SELECT id, name, path, created_at, updated_at FROM libraries WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(lib)
}

pub async fn get_default_library(pool: &DbPool) -> AppResult<Library> {
    let lib = sqlx::query_as::<_, Library>(
        "SELECT id, name, path, created_at, updated_at FROM libraries ORDER BY id ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Default library not found".to_string()))?;
    Ok(lib)
}

pub async fn upsert_book(pool: &DbPool, new_book: &NewBook) -> AppResult<(i64, bool)> {
    let existing: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM books WHERE file_path = ? LIMIT 1")
            .bind(&new_book.file_path)
            .fetch_optional(pool)
            .await?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"
            UPDATE books SET
                library_id = ?,
                title = ?,
                series = ?,
                volume = ?,
                chapter = ?,
                format = ?,
                file_size = ?,
                page_count = ?,
                cover_image_path = COALESCE(?, cover_image_path),
                summary = ?,
                genre = ?,
                language = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(new_book.library_id)
        .bind(&new_book.title)
        .bind(&new_book.series)
        .bind(&new_book.volume)
        .bind(&new_book.chapter)
        .bind(&new_book.format)
        .bind(new_book.file_size)
        .bind(new_book.page_count)
        .bind(&new_book.cover_image_path)
        .bind(&new_book.summary)
        .bind(&new_book.genre)
        .bind(&new_book.language)
        .bind(id)
        .execute(pool)
        .await?;

        Ok((id, false))
    } else {
        let res = sqlx::query(
            r#"
            INSERT INTO books (
                library_id, title, series, volume, chapter, format,
                file_path, file_size, page_count, cover_image_path,
                summary, genre, language
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(new_book.library_id)
        .bind(&new_book.title)
        .bind(&new_book.series)
        .bind(&new_book.volume)
        .bind(&new_book.chapter)
        .bind(&new_book.format)
        .bind(&new_book.file_path)
        .bind(new_book.file_size)
        .bind(new_book.page_count)
        .bind(&new_book.cover_image_path)
        .bind(&new_book.summary)
        .bind(&new_book.genre)
        .bind(&new_book.language)
        .execute(pool)
        .await?;

        Ok((res.last_insert_rowid(), true))
    }
}

pub async fn replace_book_chapters(
    pool: &DbPool,
    book_id: i64,
    chapters: &[NewChapter],
) -> AppResult<()> {
    sqlx::query("DELETE FROM book_chapters WHERE book_id = ?")
        .bind(book_id)
        .execute(pool)
        .await?;

    for ch in chapters {
        sqlx::query(
            r#"
            INSERT INTO book_chapters (book_id, chapter_index, title, content_href, word_count)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(book_id)
        .bind(ch.chapter_index)
        .bind(&ch.title)
        .bind(&ch.content_href)
        .bind(ch.word_count)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_book_by_id(pool: &DbPool, id: i64) -> AppResult<Option<Book>> {
    let book = sqlx::query_as::<_, Book>(
        r#"
        SELECT id, library_id, title, series, volume, chapter, format,
               file_path, file_size, page_count, cover_image_path,
               summary, genre, language, created_at, updated_at
        FROM books WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(book)
}

pub async fn get_book_chapters(pool: &DbPool, book_id: i64) -> AppResult<Vec<BookChapter>> {
    let chapters = sqlx::query_as::<_, BookChapter>(
        r#"
        SELECT id, book_id, chapter_index, title, content_href, word_count
        FROM book_chapters
        WHERE book_id = ?
        ORDER BY chapter_index ASC
        "#,
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(chapters)
}

pub async fn list_books(pool: &DbPool, filter: &BookFilterQuery) -> AppResult<Vec<BookSummaryDto>> {
    let limit = filter.limit.unwrap_or(100).clamp(1, 500);
    let offset = filter.offset.unwrap_or(0).max(0);

    let mut sql = String::from(
        r#"
        SELECT 
            b.id,
            b.title,
            b.series,
            b.volume,
            b.chapter,
            b.format,
            b.page_count,
            COALESCE(p.current_page, 0) as current_page,
            COALESCE(p.is_finished, 0) as is_finished,
            CASE WHEN b.cover_image_path IS NOT NULL AND b.cover_image_path != '' THEN 1 ELSE 0 END as has_cover
        FROM books b
        LEFT JOIN reading_progress p ON b.id = p.book_id
        WHERE 1=1
        "#,
    );

    if filter.library_id.is_some() {
        sql.push_str(" AND b.library_id = ?");
    }
    if filter.format.is_some() {
        sql.push_str(" AND UPPER(b.format) = UPPER(?)");
    }
    if filter.query.is_some() {
        sql.push_str(" AND (b.title LIKE ? OR b.series LIKE ?)");
    }

    sql.push_str(" ORDER BY CASE WHEN b.series != '' THEN b.series ELSE b.title END ASC, b.volume ASC, b.title ASC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            bool,
            i64,
        ),
    >(&sql);

    if let Some(lib_id) = filter.library_id {
        query = query.bind(lib_id);
    }
    if let Some(ref fmt) = filter.format {
        query = query.bind(fmt);
    }
    if let Some(ref q) = filter.query {
        let pattern = format!("%{}%", q);
        query = query.bind(pattern.clone()).bind(pattern);
    }

    query = query.bind(limit).bind(offset);

    let rows = query.fetch_all(pool).await?;

    let summaries = rows
        .into_iter()
        .map(
            |(
                id,
                title,
                series,
                volume,
                chapter,
                format,
                page_count,
                current_page,
                is_finished,
                has_cover_flag,
            )| {
                let progress_percent = if is_finished {
                    100.0
                } else if page_count > 0 {
                    ((current_page as f64 / page_count as f64) * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };

                BookSummaryDto {
                    id,
                    title,
                    series,
                    volume,
                    chapter,
                    format,
                    page_count,
                    current_page,
                    progress_percent: (progress_percent * 100.0).round() / 100.0,
                    is_finished,
                    has_cover: has_cover_flag == 1,
                }
            },
        )
        .collect();

    Ok(summaries)
}

pub async fn delete_book(pool: &DbPool, id: i64) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM books WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn get_all_book_paths_in_library(
    pool: &DbPool,
    library_id: i64,
) -> AppResult<Vec<(i64, String)>> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, file_path FROM books WHERE library_id = ?")
            .bind(library_id)
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

pub async fn delete_book_by_id(pool: &DbPool, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM books WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_storage_stats(pool: &DbPool) -> AppResult<(i64, i64)> {
    let books_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM books")
        .fetch_one(pool)
        .await?;
    let libraries_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM libraries")
        .fetch_one(pool)
        .await?;
    Ok((books_count.0, libraries_count.0))
}
