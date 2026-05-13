//! Cached "total time computed" stat for the landing page.
//!
//! Sums `compute_ms` across all metering_logs. The landing page is the front
//! door, so we don't want to hit the DB on every render. A coarse TTL (default
//! 10 minutes) is plenty — this number only grows, and a few minutes of
//! staleness on a marketing stat is invisible.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use tokio::sync::Mutex;

const DEFAULT_TTL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct TotalComputeCache {
    inner: Arc<Mutex<Option<Entry>>>,
    ttl: Duration,
}

struct Entry {
    total_ms: i64,
    fetched_at: Instant,
}

impl TotalComputeCache {
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self { inner: Arc::new(Mutex::new(None)), ttl }
    }

    /// Return the cached total compute time in milliseconds, refreshing from
    /// the DB if the cached value is missing or older than the TTL. Errors are
    /// swallowed and return the stale value (or 0 on cold start) — the landing
    /// page must not fail on stats.
    pub async fn get(&self, db: &DatabaseConnection) -> i64 {
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.as_ref()
            && entry.fetched_at.elapsed() < self.ttl
        {
            return entry.total_ms;
        }

        let total_ms = match query_total(db).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("total compute query failed: {e}");
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
            db.get_database_backend(),
            "SELECT COALESCE(SUM(compute_ms), 0) AS total FROM metering_logs",
        ))
        .await?;
    Ok(row.and_then(|r| r.try_get::<i64>("", "total").ok()).unwrap_or(0))
}

/// Humanize milliseconds for marketing copy. Picks the largest unit that
/// keeps the number readable. Thresholds are `>= 1` of each unit so values
/// near a boundary render in the unit they belong to (e.g. 36 hours →
/// "1.5 days", not "36.0 hours").
pub fn humanize_ms(ms: i64) -> String {
    if ms <= 0 {
        return "0 seconds".into();
    }
    let secs = ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    if days >= 1 {
        let frac = hours as f64 / 24.0;
        format!("{frac:.1} days")
    } else if hours >= 1 {
        let frac = mins as f64 / 60.0;
        format!("{frac:.1} hours")
    } else if mins >= 1 {
        let frac = secs as f64 / 60.0;
        format!("{frac:.1} minutes")
    } else {
        format!("{secs} seconds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::entities::metering_logs;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, Set};
    use sea_orm_migration::MigratorTrait;
    use tempfile::TempDir;

    // ── humanize_ms ───────────────────────────────────────────────────

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

    #[test]
    fn humanize_boundary_rolls_up() {
        // 36h = 1.5d. Old `>= 2` threshold rendered "36.0 hours". New
        // `>= 1` threshold renders the more natural "1.5 days".
        assert_eq!(humanize_ms(36 * 3600 * 1000), "1.5 days");
        // 90 min = 1.5h, same story.
        assert_eq!(humanize_ms(90 * 60 * 1000), "1.5 hours");
    }

    // ── Cache behavior ────────────────────────────────────────────────

    /// Set up a tempfile-backed SQLite DB with migrations applied. Returns
    /// (db, tempdir) — the caller must keep `tempdir` alive for the duration
    /// of the test, otherwise the underlying file is removed and the
    /// connection becomes unusable on the next reopen.
    async fn test_db() -> (DatabaseConnection, TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/test.db?mode=rwc", tmp.path().display());
        let db = Database::connect(&url).await.expect("connect test db");
        crate::db::migrations::Migrator::up(&db, None)
            .await
            .expect("run migrations");
        (db, tmp)
    }

    async fn insert_log(db: &DatabaseConnection, compute_ms: i64) {
        let row = metering_logs::ActiveModel {
            host_endpoint_id: Set("test-host".into()),
            model: Set("test-model".into()),
            request_type: Set("chat".into()),
            prompt_tokens: Set(0),
            completion_tokens: Set(0),
            compute_ms: Set(compute_ms),
            total_ms: Set(None),
            iroh_rtt_ms: Set(None),
            created_at: Set(chrono::Utc::now()),
            api_key_id: Set(None),
            pool_id: Set(None),
            ..Default::default()
        };
        row.insert(db).await.expect("insert metering log");
    }

    #[tokio::test]
    async fn empty_table_returns_zero() {
        let (db, _tmp) = test_db().await;
        let cache = TotalComputeCache::new();
        assert_eq!(cache.get(&db).await, 0);
    }

    #[tokio::test]
    async fn fresh_fetch_sums_rows() {
        let (db, _tmp) = test_db().await;
        insert_log(&db, 7_500).await;
        insert_log(&db, 2_500).await;

        let cache = TotalComputeCache::new();
        assert_eq!(cache.get(&db).await, 10_000);
    }

    #[tokio::test]
    async fn returns_cached_within_ttl() {
        let (db, _tmp) = test_db().await;
        insert_log(&db, 1_000).await;

        let cache = TotalComputeCache::with_ttl(Duration::from_secs(60));
        assert_eq!(cache.get(&db).await, 1_000);

        // New row should not be visible until TTL expires.
        insert_log(&db, 5_000).await;
        assert_eq!(cache.get(&db).await, 1_000);
    }

    #[tokio::test]
    async fn refreshes_after_ttl_expires() {
        let (db, _tmp) = test_db().await;
        insert_log(&db, 1_000).await;

        let cache = TotalComputeCache::with_ttl(Duration::from_millis(40));
        assert_eq!(cache.get(&db).await, 1_000);

        insert_log(&db, 2_000).await;
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(cache.get(&db).await, 3_000);
    }

    #[tokio::test]
    async fn returns_zero_on_db_error_when_cold() {
        let (db, _tmp) = test_db().await;
        // Drop the table so the next SUM query errors out.
        db.execute_unprepared("DROP TABLE metering_logs")
            .await
            .unwrap();

        let cache = TotalComputeCache::new();
        assert_eq!(cache.get(&db).await, 0);
    }

    #[tokio::test]
    async fn returns_stale_on_db_error_after_warm() {
        let (db, _tmp) = test_db().await;
        insert_log(&db, 4_242).await;

        let cache = TotalComputeCache::with_ttl(Duration::from_millis(40));
        assert_eq!(cache.get(&db).await, 4_242);

        // Force the next call past the TTL so it tries to refresh,
        // then break the underlying table so the refresh fails.
        tokio::time::sleep(Duration::from_millis(80)).await;
        db.execute_unprepared("DROP TABLE metering_logs")
            .await
            .unwrap();

        // Refresh fails; the cache should keep serving the last good value.
        assert_eq!(cache.get(&db).await, 4_242);
    }
}
