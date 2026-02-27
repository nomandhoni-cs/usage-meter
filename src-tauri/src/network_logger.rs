use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;

/// Represents a single network usage log entry for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLog {
    pub id: Option<i64>,
    pub date: String, // Format: YYYY-MM-DD
    pub uploaded_bytes: i64,
    pub downloaded_bytes: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Aggregated network statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub start_date: String,
    pub end_date: String,
    pub total_uploaded_bytes: i64,
    pub total_downloaded_bytes: i64,
    pub total_uploaded_mb: f64,
    pub total_downloaded_mb: f64,
    pub total_uploaded_gb: f64,
    pub total_downloaded_gb: f64,
    pub days_count: i64,
}

/// Time period for querying network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TimePeriod {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
    Custom {
        start_date: String,
        end_date: String,
    },
}

pub struct NetworkLogger {
    pool: SqlitePool,
}

impl NetworkLogger {
    /// Create a new NetworkLogger instance with the given database path
    pub async fn new(db_path: PathBuf) -> Result<Self, sqlx::Error> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Use file:// protocol for SQLite
        let connection_string = format!("sqlite://{}?mode=rwc", db_path.display());
        eprintln!("Connecting to database: {}", connection_string);

        let pool = SqlitePool::connect(&connection_string).await?;

        let logger = Self { pool };
        logger.init_database().await?;

        Ok(logger)
    }

    /// Initialize the database schema
    async fn init_database(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS network_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL UNIQUE,
                uploaded_bytes INTEGER NOT NULL DEFAULT 0,
                downloaded_bytes INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on date for faster queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_network_logs_date ON network_logs(date)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Log network usage for today (incremental update)
    pub async fn log_usage(
        &self,
        uploaded_bytes: i64,
        downloaded_bytes: i64,
    ) -> Result<(), sqlx::Error> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let now = Local::now().to_rfc3339();

        // Insert or update today's record
        sqlx::query(
            r#"
            INSERT INTO network_logs (date, uploaded_bytes, downloaded_bytes, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(date) DO UPDATE SET
                uploaded_bytes = uploaded_bytes + excluded.uploaded_bytes,
                downloaded_bytes = downloaded_bytes + excluded.downloaded_bytes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&today)
        .bind(uploaded_bytes)
        .bind(downloaded_bytes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get network statistics for a specific time period
    pub async fn get_stats(&self, period: TimePeriod) -> Result<NetworkStats, sqlx::Error> {
        let (start_date, end_date) = self.calculate_date_range(&period);

        let result = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT 
                COALESCE(SUM(uploaded_bytes), 0) as total_uploaded,
                COALESCE(SUM(downloaded_bytes), 0) as total_downloaded,
                COUNT(*) as days_count
            FROM network_logs
            WHERE date >= ? AND date <= ?
            "#,
        )
        .bind(&start_date)
        .bind(&end_date)
        .fetch_one(&self.pool)
        .await?;

        let (total_uploaded, total_downloaded, days_count) = result;

        Ok(NetworkStats {
            start_date,
            end_date,
            total_uploaded_bytes: total_uploaded,
            total_downloaded_bytes: total_downloaded,
            total_uploaded_mb: total_uploaded as f64 / 1_048_576.0,
            total_downloaded_mb: total_downloaded as f64 / 1_048_576.0,
            total_uploaded_gb: total_uploaded as f64 / 1_073_741_824.0,
            total_downloaded_gb: total_downloaded as f64 / 1_073_741_824.0,
            days_count,
        })
    }

    /// Get daily logs for a specific time period
    pub async fn get_daily_logs(&self, period: TimePeriod) -> Result<Vec<NetworkLog>, sqlx::Error> {
        let (start_date, end_date) = self.calculate_date_range(&period);

        let logs = sqlx::query_as::<_, (i64, String, i64, i64, String, String)>(
            r#"
            SELECT id, date, uploaded_bytes, downloaded_bytes, created_at, updated_at
            FROM network_logs
            WHERE date >= ? AND date <= ?
            ORDER BY date DESC
            "#,
        )
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs
            .into_iter()
            .map(
                |(id, date, uploaded, downloaded, created, updated)| NetworkLog {
                    id: Some(id),
                    date,
                    uploaded_bytes: uploaded,
                    downloaded_bytes: downloaded,
                    created_at: created,
                    updated_at: updated,
                },
            )
            .collect())
    }

    /// Calculate date range based on time period
    fn calculate_date_range(&self, period: &TimePeriod) -> (String, String) {
        let now = Local::now();

        match period {
            TimePeriod::Today => {
                let today = now.format("%Y-%m-%d").to_string();
                (today.clone(), today)
            }
            TimePeriod::Yesterday => {
                let yesterday = (now - chrono::Duration::days(1))
                    .format("%Y-%m-%d")
                    .to_string();
                (yesterday.clone(), yesterday)
            }
            TimePeriod::ThisWeek => {
                let start = now
                    .date_naive()
                    .week(chrono::Weekday::Mon)
                    .first_day()
                    .format("%Y-%m-%d")
                    .to_string();
                let end = now.format("%Y-%m-%d").to_string();
                (start, end)
            }
            TimePeriod::LastWeek => {
                let last_week = now - chrono::Duration::weeks(1);
                let start = last_week
                    .date_naive()
                    .week(chrono::Weekday::Mon)
                    .first_day()
                    .format("%Y-%m-%d")
                    .to_string();
                let end = last_week
                    .date_naive()
                    .week(chrono::Weekday::Mon)
                    .last_day()
                    .format("%Y-%m-%d")
                    .to_string();
                (start, end)
            }
            TimePeriod::ThisMonth => {
                let start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string();
                let end = now.format("%Y-%m-%d").to_string();
                (start, end)
            }
            TimePeriod::LastMonth => {
                let last_month = if now.month() == 1 {
                    NaiveDate::from_ymd_opt(now.year() - 1, 12, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(now.year(), now.month() - 1, 1).unwrap()
                };
                let start = last_month.format("%Y-%m-%d").to_string();
                let end = (last_month
                    + chrono::Duration::days(
                        days_in_month(last_month.year(), last_month.month()) as i64 - 1,
                    ))
                .format("%Y-%m-%d")
                .to_string();
                (start, end)
            }
            TimePeriod::ThisYear => {
                let start = NaiveDate::from_ymd_opt(now.year(), 1, 1)
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string();
                let end = now.format("%Y-%m-%d").to_string();
                (start, end)
            }
            TimePeriod::LastYear => {
                let start = NaiveDate::from_ymd_opt(now.year() - 1, 1, 1)
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string();
                let end = NaiveDate::from_ymd_opt(now.year() - 1, 12, 31)
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string();
                (start, end)
            }
            TimePeriod::Custom {
                start_date,
                end_date,
            } => (start_date.clone(), end_date.clone()),
        }
    }

    /// Delete logs older than a specified number of days
    pub async fn cleanup_old_logs(&self, days_to_keep: i64) -> Result<u64, sqlx::Error> {
        let cutoff_date = (Local::now() - chrono::Duration::days(days_to_keep))
            .format("%Y-%m-%d")
            .to_string();

        let result = sqlx::query(
            r#"
            DELETE FROM network_logs
            WHERE date < ?
            "#,
        )
        .bind(&cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Close the database connection
    #[allow(dead_code)]
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Helper function to get the number of days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    }
    .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
    .num_days() as u32
}
