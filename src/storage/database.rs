use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::Result;
use std::path::Path;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        if database_url.starts_with("sqlite:") {
            let path_str = database_url.strip_prefix("sqlite:").unwrap();
            let path = Path::new(path_str);
            ensure_database_exists(path).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn initialize(&self) -> Result<()> {
        self.create_tables().await?;
        Ok(())
    }

    async fn create_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                status TEXT NOT NULL,
                profile TEXT NOT NULL,
                target TEXT NOT NULL,
                environment TEXT NOT NULL,
                config TEXT NOT NULL,
                phases TEXT NOT NULL,
                findings_count TEXT NOT NULL,
                metrics TEXT NOT NULL,
                checkpoint TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS findings (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                title TEXT NOT NULL,
                vuln_class TEXT NOT NULL,
                severity TEXT NOT NULL,
                cvss_v4_score TEXT NOT NULL,
                status TEXT NOT NULL,
                target TEXT NOT NULL,
                evidence TEXT NOT NULL,
                root_cause TEXT,
                remediation TEXT NOT NULL,
                environment TEXT NOT NULL,
                authorization TEXT NOT NULL,
                retest_history TEXT NOT NULL,
                human_review TEXT,
                FOREIGN KEY (run_id) REFERENCES runs(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS roe (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                authorized_by TEXT NOT NULL,
                scope TEXT NOT NULL,
                constraints TEXT NOT NULL,
                approved_actions TEXT NOT NULL,
                forbidden_actions TEXT NOT NULL,
                contact_info TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_findings_run_id ON findings(run_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_findings_status ON findings(status)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}

pub async fn ensure_database_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(())
}
