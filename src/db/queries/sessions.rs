use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    HeartbeatResponse, ReadingSession, ReadingStatsDto, RecentDayStatDto, RecentSessionDto,
    StartSessionRequest, StartSessionResponse, TodayStatsDto,
};
use chrono::{Local, NaiveDate};

pub async fn start_session(
    pool: &DbPool,
    req: &StartSessionRequest,
) -> AppResult<StartSessionResponse> {
    // Close any dangling active sessions
    sqlx::query(
        "UPDATE reading_sessions SET is_active = 0, ended_at = CURRENT_TIMESTAMP WHERE is_active = 1",
    )
    .execute(pool)
    .await?;

    let start_page = req.current_page.unwrap_or(0);
    let is_pomodoro = req.is_pomodoro.unwrap_or(false);
    let goal_minutes = req
        .goal_minutes
        .unwrap_or(if is_pomodoro { 25 } else { 30 });

    let res = sqlx::query(
        r#"
        INSERT INTO reading_sessions (
            book_id, started_at, last_heartbeat_at, active_seconds,
            start_page, end_page, pages_read, is_pomodoro, goal_minutes, is_active
        ) VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0, ?, ?, 0, ?, ?, 1)
        "#,
    )
    .bind(req.book_id)
    .bind(start_page)
    .bind(start_page)
    .bind(is_pomodoro)
    .bind(goal_minutes)
    .execute(pool)
    .await?;

    let session_id = res.last_insert_rowid();

    let started_at: (String,) =
        sqlx::query_as("SELECT started_at FROM reading_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_one(pool)
            .await?;

    Ok(StartSessionResponse {
        session_id,
        started_at: started_at.0,
        goal_minutes,
    })
}

pub async fn heartbeat(
    pool: &DbPool,
    session_id: i64,
    current_page: i64,
    delta_seconds: i64,
) -> AppResult<HeartbeatResponse> {
    let session = sqlx::query_as::<_, ReadingSession>(
        "SELECT id, book_id, started_at, last_heartbeat_at, ended_at, active_seconds, start_page, end_page, pages_read, is_pomodoro, goal_minutes, is_active FROM reading_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Session {} not found", session_id)))?;

    if !session.is_active {
        return Ok(HeartbeatResponse {
            status: "paused".to_string(),
            active_seconds: session.active_seconds,
            pages_read: session.pages_read,
        });
    }

    let delta_secs = delta_seconds.clamp(0, 300);
    let new_active_secs = session.active_seconds + delta_secs;
    let new_pages_read = session
        .pages_read
        .max((current_page - session.start_page).max(0));
    let delta_pages = (new_pages_read - session.pages_read).max(0);

    sqlx::query(
        r#"
        UPDATE reading_sessions
        SET last_heartbeat_at = CURRENT_TIMESTAMP,
            active_seconds = ?,
            end_page = ?,
            pages_read = ?
        WHERE id = ?
        "#,
    )
    .bind(new_active_secs)
    .bind(current_page)
    .bind(new_pages_read)
    .bind(session_id)
    .execute(pool)
    .await?;

    // Update daily_stats rollup
    let today_str = Local::now().format("%Y-%m-%d").to_string();

    sqlx::query(
        r#"
        INSERT INTO daily_stats (date_str, total_seconds, pages_read, books_touched, sessions_count)
        VALUES (?, ?, ?, 1, 1)
        ON CONFLICT(date_str) DO UPDATE SET
            total_seconds = total_seconds + excluded.total_seconds,
            pages_read = pages_read + excluded.pages_read
        "#,
    )
    .bind(&today_str)
    .bind(delta_secs)
    .bind(delta_pages)
    .execute(pool)
    .await?;

    Ok(HeartbeatResponse {
        status: "active".to_string(),
        active_seconds: new_active_secs,
        pages_read: new_pages_read,
    })
}

pub async fn pause_session(pool: &DbPool, session_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE reading_sessions SET last_heartbeat_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn stop_session(
    pool: &DbPool,
    session_id: i64,
    final_page: Option<i64>,
) -> AppResult<()> {
    if let Some(fp) = final_page {
        sqlx::query(
            r#"
            UPDATE reading_sessions
            SET is_active = 0,
                ended_at = CURRENT_TIMESTAMP,
                last_heartbeat_at = CURRENT_TIMESTAMP,
                end_page = ?,
                pages_read = MAX(pages_read, MAX(0, ? - start_page))
            WHERE id = ?
            "#,
        )
        .bind(fp)
        .bind(fp)
        .bind(session_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE reading_sessions SET is_active = 0, ended_at = CURRENT_TIMESTAMP, last_heartbeat_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_reading_stats(pool: &DbPool, book_id: Option<i64>) -> AppResult<ReadingStatsDto> {
    let (total_seconds, total_pages): (i64, i64) = if let Some(b_id) = book_id {
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT SUM(active_seconds), SUM(pages_read) FROM reading_sessions WHERE book_id = ?",
        )
        .bind(b_id)
        .fetch_one(pool)
        .await?;
        (row.0.unwrap_or(0), row.1.unwrap_or(0))
    } else {
        let row: (Option<i64>, Option<i64>) =
            sqlx::query_as("SELECT SUM(active_seconds), SUM(pages_read) FROM reading_sessions")
                .fetch_one(pool)
                .await?;
        (row.0.unwrap_or(0), row.1.unwrap_or(0))
    };

    let total_reading_hours = ((total_seconds as f64 / 3600.0) * 10.0).round() / 10.0;
    let pages_per_minute = if total_seconds >= 60 {
        let ppm = total_pages as f64 / (total_seconds as f64 / 60.0);
        (ppm * 100.0).round() / 100.0
    } else {
        0.0
    };

    // Today's stats
    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let today_row: Option<(i64, i64)> =
        sqlx::query_as("SELECT total_seconds, pages_read FROM daily_stats WHERE date_str = ?")
            .bind(&today_str)
            .fetch_optional(pool)
            .await?;

    let (today_secs, today_pages) = today_row.unwrap_or((0, 0));
    let today_minutes = today_secs / 60;
    let goal_minutes = 30;

    let today = TodayStatsDto {
        seconds: today_secs,
        minutes: today_minutes,
        pages: today_pages,
        goal_minutes,
        goal_achieved: today_minutes >= goal_minutes,
    };

    // Recent days (last 14 days)
    let recent_rows: Vec<(String, i64, i64)> = sqlx::query_as(
        r#"
        SELECT date_str, total_seconds, pages_read
        FROM daily_stats
        ORDER BY date_str DESC
        LIMIT 14
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut recent_days: Vec<RecentDayStatDto> = recent_rows
        .into_iter()
        .map(|(date, secs, p)| RecentDayStatDto {
            date,
            minutes: secs / 60,
            pages: p,
        })
        .collect();
    recent_days.reverse();

    // Streaks calculation from daily_stats
    let date_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT date_str FROM daily_stats
        WHERE total_seconds >= 60 OR pages_read >= 1
        ORDER BY date_str DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let active_dates: Vec<NaiveDate> = date_rows
        .into_iter()
        .filter_map(|(d,)| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .collect();

    let (current_streak, longest_streak) =
        calculate_streaks(&active_dates, Local::now().date_naive());

    // Recent sessions (last 10)
    let session_rows: Vec<(i64, i64, String, String, i64, i64)> = sqlx::query_as(
        r#"
        SELECT s.id, s.book_id, b.title, s.started_at, s.active_seconds, s.pages_read
        FROM reading_sessions s
        JOIN books b ON s.book_id = b.id
        ORDER BY s.started_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;

    let recent_sessions = session_rows
        .into_iter()
        .map(|(id, b_id, title, started_at, secs, pages)| {
            let mins = secs / 60;
            let ppm = if secs >= 60 {
                ((pages as f64 / (secs as f64 / 60.0)) * 100.0).round() / 100.0
            } else {
                0.0
            };
            RecentSessionDto {
                id,
                book_id: b_id,
                book_title: title,
                date: started_at,
                active_minutes: mins,
                pages_read: pages,
                ppm,
            }
        })
        .collect();

    Ok(ReadingStatsDto {
        current_streak_days: current_streak,
        longest_streak_days: longest_streak,
        total_reading_seconds: total_seconds,
        total_reading_hours,
        total_pages_read: total_pages,
        pages_per_minute,
        today,
        recent_days,
        recent_sessions,
    })
}

pub fn calculate_streaks(dates: &[NaiveDate], today: NaiveDate) -> (i64, i64) {
    if dates.is_empty() {
        return (0, 0);
    }

    // Dates are sorted descending
    let yesterday = today.pred_opt().unwrap_or(today);

    // Current streak
    let mut current_streak = 0;
    if let Some(&first) = dates.first() {
        if first == today || first == yesterday {
            let mut expected = first;
            for &d in dates {
                if d == expected {
                    current_streak += 1;
                    if let Some(prev) = expected.pred_opt() {
                        expected = prev;
                    } else {
                        break;
                    }
                } else if d < expected {
                    break;
                }
            }
        }
    }

    // Longest streak
    let mut longest_streak = 0;
    let mut temp_streak = 0;
    let mut prev_date: Option<NaiveDate> = None;

    // Scan dates from oldest to newest
    let mut sorted_dates = dates.to_vec();
    sorted_dates.sort();
    sorted_dates.dedup();

    for d in sorted_dates {
        if let Some(prev) = prev_date {
            if let Some(next_day) = prev.succ_opt() {
                if d == next_day {
                    temp_streak += 1;
                } else {
                    temp_streak = 1;
                }
            } else {
                temp_streak = 1;
            }
        } else {
            temp_streak = 1;
        }
        if temp_streak > longest_streak {
            longest_streak = temp_streak;
        }
        prev_date = Some(d);
    }

    (current_streak, longest_streak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streak_calculation() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        let d4 = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let d5 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        let dates = vec![d1, d2, d3, d4, d5];
        let (current, longest) = calculate_streaks(&dates, today);
        assert_eq!(current, 3);
        assert_eq!(longest, 3);
    }
}
