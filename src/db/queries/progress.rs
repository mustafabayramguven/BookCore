use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::{ProgressUpdateDto, ReadingProgress};

pub async fn get_progress(pool: &DbPool, book_id: i64) -> AppResult<Option<ReadingProgress>> {
    let progress = sqlx::query_as::<_, ReadingProgress>(
        r#"
        SELECT id, book_id, current_page, scroll_anchor, is_finished, total_reads, last_read_at
        FROM reading_progress
        WHERE book_id = ?
        "#,
    )
    .bind(book_id)
    .fetch_optional(pool)
    .await?;
    Ok(progress)
}

pub async fn upsert_progress(
    pool: &DbPool,
    book_id: i64,
    update: &ProgressUpdateDto,
) -> AppResult<ReadingProgress> {
    let existing = get_progress(pool, book_id).await?;

    if let Some(prev) = existing {
        let is_fin = update.is_finished.unwrap_or(prev.is_finished);
        let inc_reads = if is_fin && !prev.is_finished { 1 } else { 0 };

        sqlx::query(
            r#"
            UPDATE reading_progress
            SET current_page = ?,
                scroll_anchor = COALESCE(?, scroll_anchor),
                is_finished = ?,
                total_reads = total_reads + ?,
                last_read_at = CURRENT_TIMESTAMP
            WHERE book_id = ?
            "#,
        )
        .bind(update.page)
        .bind(&update.scroll_anchor)
        .bind(is_fin)
        .bind(inc_reads)
        .bind(book_id)
        .execute(pool)
        .await?;
    } else {
        let is_fin = update.is_finished.unwrap_or(false);
        let total_reads = if is_fin { 1 } else { 0 };

        sqlx::query(
            r#"
            INSERT INTO reading_progress (book_id, current_page, scroll_anchor, is_finished, total_reads, last_read_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(book_id)
        .bind(update.page)
        .bind(&update.scroll_anchor)
        .bind(is_fin)
        .bind(total_reads)
        .execute(pool)
        .await?;
    }

    let result = get_progress(pool, book_id).await?.unwrap();
    Ok(result)
}
