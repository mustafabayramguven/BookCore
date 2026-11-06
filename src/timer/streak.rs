#![allow(dead_code)]

use chrono::NaiveDate;

pub fn calculate_ppm(pages_read: i64, active_seconds: i64) -> f64 {
    if active_seconds < 60 || pages_read <= 0 {
        return 0.0;
    }
    let minutes = active_seconds as f64 / 60.0;
    let ppm = pages_read as f64 / minutes;
    (ppm * 100.0).round() / 100.0
}

pub fn calculate_streak_days(dates: &[NaiveDate], today: NaiveDate) -> (i64, i64) {
    crate::db::queries::sessions::calculate_streaks(dates, today)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_ppm() {
        assert_eq!(calculate_ppm(10, 600), 1.0); // 10 pages in 10 minutes = 1.0 ppm
        assert_eq!(calculate_ppm(15, 600), 1.5);
        assert_eq!(calculate_ppm(5, 30), 0.0); // Less than 60 seconds returns 0.0
    }
}
