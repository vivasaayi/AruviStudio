use std::fmt::Display;
use std::time::Instant;
use tracing::{debug, warn};

const SLOW_PERSISTENCE_QUERY_MS: i64 = 250;

pub fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

pub fn record_persistence_query(query_name: &str, duration_ms: i64, row_count: Option<usize>) {
    if duration_ms >= SLOW_PERSISTENCE_QUERY_MS {
        warn!(
            query_name,
            duration_ms,
            row_count,
            slow_query_threshold_ms = SLOW_PERSISTENCE_QUERY_MS,
            "slow persistence query"
        );
    } else {
        debug!(
            query_name,
            duration_ms, row_count, "persistence query completed"
        );
    }
}

pub fn record_persistence_query_error(query_name: &str, duration_ms: i64, error: impl Display) {
    warn!(
        query_name,
        duration_ms,
        error = %error,
        "persistence query failed"
    );
}
