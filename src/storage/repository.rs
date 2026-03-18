use sqlx::SqlitePool;
use anyhow::Result;
use uuid::Uuid;
use crate::models::{Finding, RunState, RulesOfEngagement, FindingStatus, Severity};

pub struct FindingRepository {
    pool: SqlitePool,
}

impl FindingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, finding: &Finding) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO findings (
                id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                status, target, evidence, root_cause, remediation, environment,
                authorization, retest_history, human_review
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(finding.id.to_string())
        .bind(finding.run_id.to_string())
        .bind(finding.timestamp.to_rfc3339())
        .bind(&finding.title)
        .bind(serde_json::to_string(&finding.vuln_class)?)
        .bind(finding.severity.as_str())
        .bind(serde_json::to_string(&finding.cvss_v4_score)?)
        .bind(finding.status.as_str())
        .bind(serde_json::to_string(&finding.target)?)
        .bind(serde_json::to_string(&finding.evidence)?)
        .bind(serde_json::to_string(&finding.root_cause)?)
        .bind(serde_json::to_string(&finding.remediation)?)
        .bind(serde_json::to_string(&finding.environment)?)
        .bind(serde_json::to_string(&finding.authorization)?)
        .bind(serde_json::to_string(&finding.retest_history)?)
        .bind(serde_json::to_string(&finding.human_review)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update(&self, finding: &Finding) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE findings SET
                timestamp = ?, title = ?, vuln_class = ?, severity = ?, cvss_v4_score = ?,
                status = ?, target = ?, evidence = ?, root_cause = ?, remediation = ?,
                environment = ?, authorization = ?, retest_history = ?, human_review = ?
            WHERE id = ?
            "#,
        )
        .bind(finding.timestamp.to_rfc3339())
        .bind(&finding.title)
        .bind(serde_json::to_string(&finding.vuln_class)?)
        .bind(finding.severity.as_str())
        .bind(serde_json::to_string(&finding.cvss_v4_score)?)
        .bind(finding.status.as_str())
        .bind(serde_json::to_string(&finding.target)?)
        .bind(serde_json::to_string(&finding.evidence)?)
        .bind(serde_json::to_string(&finding.root_cause)?)
        .bind(serde_json::to_string(&finding.remediation)?)
        .bind(serde_json::to_string(&finding.environment)?)
        .bind(serde_json::to_string(&finding.authorization)?)
        .bind(serde_json::to_string(&finding.retest_history)?)
        .bind(serde_json::to_string(&finding.human_review)?)
        .bind(finding.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Finding>> {
        let row = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                   status, target, evidence, root_cause, remediation, environment,
                   authorization, retest_history, human_review
            FROM findings WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_finding(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_run_id(&self, run_id: Uuid) -> Result<Vec<Finding>> {
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                   status, target, evidence, root_cause, remediation, environment,
                   authorization, retest_history, human_review
            FROM findings WHERE run_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_finding(row))
            .collect()
    }

    pub async fn find_by_status(&self, status: FindingStatus) -> Result<Vec<Finding>> {
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                   status, target, evidence, root_cause, remediation, environment,
                   authorization, retest_history, human_review
            FROM findings WHERE status = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_finding(row))
            .collect()
    }

    pub async fn find_by_severity(&self, severity: Severity) -> Result<Vec<Finding>> {
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                   status, target, evidence, root_cause, remediation, environment,
                   authorization, retest_history, human_review
            FROM findings WHERE severity = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(severity.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_finding(row))
            .collect()
    }
    
    pub async fn list_all(&self) -> Result<Vec<Finding>> {
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, title, vuln_class, severity, cvss_v4_score,
                   status, target, evidence, root_cause, remediation, environment,
                   authorization, retest_history, human_review
            FROM findings
            ORDER BY timestamp DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_finding(row))
            .collect()
    }

    fn row_to_finding(&self, row: sqlx::sqlite::SqliteRow) -> Result<Finding> {
        use sqlx::Row;

        Ok(Finding {
            id: Uuid::parse_str(row.get("id"))?,
            run_id: Uuid::parse_str(row.get("run_id"))?,
            timestamp: chrono::DateTime::parse_from_rfc3339(row.get("timestamp"))?.with_timezone(&chrono::Utc),
            title: row.get("title"),
            description: row.try_get("description").unwrap_or_else(|_| String::new()),
            vuln_class: serde_json::from_str(row.get("vuln_class"))?,
            severity: match row.get::<&str, _>("severity") {
                "critical" => Severity::Critical,
                "high" => Severity::High,
                "medium" => Severity::Medium,
                "low" => Severity::Low,
                _ => Severity::Info,
            },
            confidence: row.try_get("confidence").unwrap_or(0.0),
            cvss_v4_score: serde_json::from_str(row.get("cvss_v4_score"))?,
            status: match row.get::<&str, _>("status") {
                "confirmed" => FindingStatus::Confirmed,
                "unconfirmed" => FindingStatus::Unconfirmed,
                "needs_review" => FindingStatus::NeedsReview,
                "fixed" => FindingStatus::Fixed,
                _ => FindingStatus::WontFix,
            },
            target: serde_json::from_str(row.get("target"))?,
            evidence: serde_json::from_str(row.get("evidence"))?,
            root_cause: serde_json::from_str(row.get("root_cause"))?,
            remediation: serde_json::from_str(row.get("remediation"))?,
            environment: serde_json::from_str(row.get("environment"))?,
            authorization: serde_json::from_str(row.get("authorization"))?,
            retest_history: serde_json::from_str(row.get("retest_history"))?,
            human_review: serde_json::from_str(row.get("human_review"))?,
        })
    }
}

pub struct RunStateRepository {
    pool: SqlitePool,
}

impl RunStateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, run: &RunState) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO runs (
                id, created_at, updated_at, status, profile, target, environment,
                config, phases, findings_count, metrics, checkpoint
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(run.id.to_string())
        .bind(run.created_at.to_rfc3339())
        .bind(run.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&run.status)?)
        .bind(run.profile.as_str())
        .bind(&run.target)
        .bind(serde_json::to_string(&run.environment)?)
        .bind(serde_json::to_string(&run.config)?)
        .bind(serde_json::to_string(&run.phases)?)
        .bind(serde_json::to_string(&run.findings_count)?)
        .bind(serde_json::to_string(&run.metrics)?)
        .bind(serde_json::to_string(&run.checkpoint)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update(&self, run: &RunState) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE runs SET
                updated_at = ?, status = ?, config = ?, phases = ?,
                findings_count = ?, metrics = ?, checkpoint = ?
            WHERE id = ?
            "#,
        )
        .bind(run.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&run.status)?)
        .bind(serde_json::to_string(&run.config)?)
        .bind(serde_json::to_string(&run.phases)?)
        .bind(serde_json::to_string(&run.findings_count)?)
        .bind(serde_json::to_string(&run.metrics)?)
        .bind(serde_json::to_string(&run.checkpoint)?)
        .bind(run.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<RunState>> {
        let row = sqlx::query(
            r#"
            SELECT id, created_at, updated_at, status, profile, target, environment,
                   config, phases, findings_count, metrics, checkpoint
            FROM runs WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_run_state(row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn list_all(&self) -> Result<Vec<RunState>> {
        let rows = sqlx::query(
            r#"
            SELECT id, created_at, updated_at, status, profile, target, environment,
                   config, phases, findings_count, metrics, checkpoint
            FROM runs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_run_state(row))
            .collect()
    }

    fn row_to_run_state(&self, row: sqlx::sqlite::SqliteRow) -> Result<RunState> {
        use sqlx::Row;

        Ok(RunState {
            id: Uuid::parse_str(row.get("id"))?,
            created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(row.get("updated_at"))?.with_timezone(&chrono::Utc),
            status: serde_json::from_str(row.get("status"))?,
            profile: match row.get::<&str, _>("profile") {
                "web" => crate::models::RunProfile::Web,
                "api" => crate::models::RunProfile::Api,
                "code" => crate::models::RunProfile::Code,
                _ => crate::models::RunProfile::Full,
            },
            target: row.get("target"),
            environment: serde_json::from_str(row.get("environment"))?,
            config: serde_json::from_str(row.get("config"))?,
            phases: serde_json::from_str(row.get("phases"))?,
            findings_count: serde_json::from_str(row.get("findings_count"))?,
            metrics: serde_json::from_str(row.get("metrics"))?,
            checkpoint: serde_json::from_str(row.get("checkpoint"))?,
        })
    }
}

pub struct RoeRepository {
    pool: SqlitePool,
}

impl RoeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, roe: &RulesOfEngagement) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO roe (
                id, created_at, authorized_by, scope, constraints,
                approved_actions, forbidden_actions, contact_info
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&roe.id)
        .bind(roe.created_at.to_rfc3339())
        .bind(&roe.authorized_by)
        .bind(serde_json::to_string(&roe.scope)?)
        .bind(serde_json::to_string(&roe.constraints)?)
        .bind(serde_json::to_string(&roe.approved_actions)?)
        .bind(serde_json::to_string(&roe.forbidden_actions)?)
        .bind(serde_json::to_string(&roe.contact_info)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<RulesOfEngagement>> {
        let row = sqlx::query(
            r#"
            SELECT id, created_at, authorized_by, scope, constraints,
                   approved_actions, forbidden_actions, contact_info
            FROM roe WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_roe(row)?))
        } else {
            Ok(None)
        }
    }

    fn row_to_roe(&self, row: sqlx::sqlite::SqliteRow) -> Result<RulesOfEngagement> {
        use sqlx::Row;

        Ok(RulesOfEngagement {
            id: row.get("id"),
            created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&chrono::Utc),
            authorized_by: row.get("authorized_by"),
            scope: serde_json::from_str(row.get("scope"))?,
            constraints: serde_json::from_str(row.get("constraints"))?,
            approved_actions: serde_json::from_str(row.get("approved_actions"))?,
            forbidden_actions: serde_json::from_str(row.get("forbidden_actions"))?,
            contact_info: serde_json::from_str(row.get("contact_info"))?,
        })
    }
}
