#![allow(dead_code)]

use chrono::{DateTime, Utc};

pub const DEFAULT_SESSION_TIMEOUT_SECONDS: i64 = 600; // 10 minutes
pub const MAX_DELTA_HEARTBEAT_SECONDS: i64 = 120; // Max acceptable tick per heartbeat

pub fn sanitize_heartbeat_delta(delta: i64) -> i64 {
    delta.clamp(0, MAX_DELTA_HEARTBEAT_SECONDS)
}

pub fn calculate_pages_turned(start_page: i64, current_page: i64) -> i64 {
    (current_page - start_page).max(0)
}

pub fn is_session_stale(last_heartbeat_rfc3339: &str, timeout_seconds: i64) -> bool {
    if let Ok(last_hb) = DateTime::parse_from_rfc3339(last_heartbeat_rfc3339) {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(last_hb.with_timezone(&Utc));
        elapsed.num_seconds() > timeout_seconds
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_delta() {
        assert_eq!(sanitize_heartbeat_delta(15), 15);
        assert_eq!(sanitize_heartbeat_delta(-5), 0);
        assert_eq!(sanitize_heartbeat_delta(500), MAX_DELTA_HEARTBEAT_SECONDS);
    }

    #[test]
    fn test_calculate_pages() {
        assert_eq!(calculate_pages_turned(10, 25), 15);
        assert_eq!(calculate_pages_turned(25, 10), 0);
    }
}
