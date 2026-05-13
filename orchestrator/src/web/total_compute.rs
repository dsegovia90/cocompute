//! Cached "total time computed" stat for the landing page.
//!
//! Sums `compute_ms` across all metering_logs. The landing page is the front
//! door, so we don't want to hit the DB on every render. A coarse TTL (default
//! 10 minutes) is plenty — this number only grows, and a few minutes of
//! staleness on a marketing stat is invisible.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tokio::sync::Mutex;

const TTL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct TotalComputeCache {
    inner: Arc<Mutex<Option<Entry>>>,
}

struct Entry {
    total_ms: i64,
    fetched_at: Instant,
}

impl TotalComputeCache {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }

    /// Return the cached total compute time in milliseconds, refreshing from
    /// the DB if the cached value is missing or older than the TTL. Errors are
    /// swallowed and return 0 — the landing page must not fail on stats.
    pub async fn get(&self, db: &DatabaseConnection) -> i64 {
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.as_ref()
            && entry.fetched_at.elapsed() < TTL
        {
            return entry.total_ms;
        }

        let total_ms = match query_total(db).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("total compute query failed: {e}");
                // Keep serving the stale value if we have one.
                return guard.as_ref().map(|e| e.total_ms).unwrap_or(0);
            }
        };

        *guard = Some(Entry { total_ms, fetched_at: Instant::now() });
        total_ms
    }
}

impl Default for TotalComputeCache {
    fn default() -> Self {
        Self::new()
    }
}

async fn query_total(db: &DatabaseConnection) -> Result<i64, sea_orm::DbErr> {
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COALESCE(SUM(compute_ms), 0) AS total FROM metering_logs",
        ))
        .await?;
    Ok(row.and_then(|r| r.try_get::<i64>("", "total").ok()).unwrap_or(0))
}

/// Humanize milliseconds for marketing copy. Picks the largest unit that
/// keeps the number readable.
pub fn humanize_ms(ms: i64) -> String {
    if ms <= 0 {
        return "0 seconds".into();
    }
    let secs = ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if days >= 2 {
        let frac = hours as f64 / 24.0;
        format!("{frac:.1} days")
    } else if hours >= 2 {
        let frac = mins as f64 / 60.0;
        format!("{frac:.1} hours")
    } else if mins >= 2 {
        let frac = secs as f64 / 60.0;
        format!("{frac:.1} minutes")
    } else {
        format!("{secs} seconds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_zero() {
        assert_eq!(humanize_ms(0), "0 seconds");
        assert_eq!(humanize_ms(-5), "0 seconds");
    }

    #[test]
    fn humanize_seconds() {
        assert_eq!(humanize_ms(45_000), "45 seconds");
    }

    #[test]
    fn humanize_minutes() {
        assert_eq!(humanize_ms(150_000), "2.5 minutes");
    }

    #[test]
    fn humanize_hours() {
        // 2.5 hours = 9_000_000 ms
        assert_eq!(humanize_ms(9_000_000), "2.5 hours");
    }

    #[test]
    fn humanize_days() {
        // 3 days = 259_200_000 ms
        assert_eq!(humanize_ms(259_200_000), "3.0 days");
    }
}
