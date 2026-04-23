#[cfg(feature = "ssr")]
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

#[cfg(feature = "ssr")]
pub type DbPool = SqlitePool;

#[cfg(feature = "ssr")]
pub async fn init_db(database_url: &str) -> DbPool {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to SQLite database");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_type TEXT NOT NULL,
            target TEXT NOT NULL,
            target_source TEXT NOT NULL DEFAULT 'url',
            status TEXT NOT NULL DEFAULT 'pending',
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_seconds INTEGER,
            total_findings INTEGER NOT NULL DEFAULT 0,
            critical_count INTEGER NOT NULL DEFAULT 0,
            high_count INTEGER NOT NULL DEFAULT 0,
            medium_count INTEGER NOT NULL DEFAULT 0,
            low_count INTEGER NOT NULL DEFAULT 0,
            info_count INTEGER NOT NULL DEFAULT 0,
            tools_run TEXT,
            file_tree TEXT,
            current_tool TEXT,
            tools_total INTEGER,
            tools_completed INTEGER
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create scan_jobs table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS findings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_job_id INTEGER NOT NULL,
            tool TEXT NOT NULL,
            severity TEXT NOT NULL DEFAULT 'info',
            title TEXT NOT NULL,
            description TEXT,
            file_path TEXT,
            line_number INTEGER,
            cwe_id TEXT,
            cvss_score REAL,
            raw_output TEXT,
            recommendation TEXT,
            text_range_start INTEGER,
            text_range_end INTEGER,
            status TEXT,
            author TEXT,
            rule_url TEXT,
            data_flow TEXT,
            issue_type TEXT,
            FOREIGN KEY (scan_job_id) REFERENCES scan_jobs(id)
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create findings table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_job_id INTEGER NOT NULL,
            format TEXT NOT NULL DEFAULT 'html',
            file_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            emailed INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (scan_job_id) REFERENCES scan_jobs(id)
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create reports table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_job_id INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL DEFAULT 'info',
            tool TEXT,
            message TEXT NOT NULL,
            FOREIGN KEY (scan_job_id) REFERENCES scan_jobs(id)
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create scan_logs table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create settings table");

    // Migration: add issue_type column if it doesn't exist yet (for existing DBs)
    let _ = sqlx::query("ALTER TABLE findings ADD COLUMN issue_type TEXT")
        .execute(&pool)
        .await;

    pool
}

// ── Query helpers ──

#[cfg(feature = "ssr")]
pub async fn get_setting(pool: &DbPool, key: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_default()
}

#[cfg(feature = "ssr")]
pub async fn set_setting(pool: &DbPool, key: &str, value: &str) {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .ok();
}

#[cfg(feature = "ssr")]
pub async fn insert_scan_log(pool: &DbPool, scan_job_id: i64, level: &str, tool: Option<&str>, message: &str) {
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO scan_logs (scan_job_id, timestamp, level, tool, message) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(scan_job_id)
    .bind(&ts)
    .bind(level)
    .bind(tool)
    .bind(message)
    .execute(pool)
    .await
    .ok();
}
