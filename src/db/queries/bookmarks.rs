use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::{Bookmark, CreateBookmarkDto};

pub async fn list_bookmarks(pool: &DbPool, book_id: i64) -> AppResult<Vec<Bookmark>> {
    let bookmarks = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT id, book_id, page_number, title, xpath, created_at
        FROM bookmarks
        WHERE book_id = ?
        ORDER BY page_number ASC, created_at DESC
        "#,
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(bookmarks)
}

pub async fn create_bookmark(
    pool: &DbPool,
    book_id: i64,
    dto: &CreateBookmarkDto,
) -> AppResult<Bookmark> {
    let res = sqlx::query(
        r#"
        INSERT INTO bookmarks (book_id, page_number, title, xpath, created_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(book_id)
    .bind(dto.page_number)
    .bind(&dto.title)
    .bind(&dto.xpath)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();

    let bookmark = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT id, book_id, page_number, title, xpath, created_at
        FROM bookmarks
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(bookmark)
}

pub async fn delete_bookmark(pool: &DbPool, bookmark_id: i64) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM bookmarks WHERE id = ?")
        .bind(bookmark_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
