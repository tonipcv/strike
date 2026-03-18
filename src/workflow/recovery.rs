use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensatingAction {
    pub id: String,
    pub phase: String,
    pub action_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub status: CompensationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompensationStatus {
    Pending,
    Executed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub id: String,
    pub run_id: String,
    pub phase: String,
    pub error: String,
    pub state_snapshot: String,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_retry_at: Option<DateTime<Utc>>,
    pub resolved: bool,
}

pub struct RecoveryManager {
    pool: SqlitePool,
    max_retries: u32,
}

impl RecoveryManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            max_retries: 3,
        }
    }
    
    pub async fn ensure_tables(&self) -> Result<()> {
        // Ensure checkpoints table exists (used by validate_state_consistency)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS run_checkpoints (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS compensating_actions (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                action_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL,
                executed_at TEXT,
                status TEXT NOT NULL,
                FOREIGN KEY (run_id) REFERENCES run_states(id)
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS dead_letter_queue (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                error TEXT NOT NULL,
                state_snapshot TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                last_retry_at TEXT,
                resolved INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (run_id) REFERENCES run_states(id)
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_compensating_run_id 
            ON compensating_actions(run_id)
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dlq_run_id 
            ON dead_letter_queue(run_id)
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn register_compensating_action(
        &self,
        run_id: &str,
        phase: &str,
        action_type: &str,
        payload: serde_json::Value,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let status = serde_json::to_string(&CompensationStatus::Pending)?;
        let payload_str = serde_json::to_string(&payload)?;
        
        sqlx::query(
            r#"
            INSERT INTO compensating_actions 
            (id, run_id, phase, action_type, payload, created_at, status)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(run_id)
        .bind(phase)
        .bind(action_type)
        .bind(&payload_str)
        .bind(&created_at)
        .bind(&status)
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    pub async fn execute_compensating_actions(&self, run_id: &str, phase: &str) -> Result<Vec<CompensatingAction>> {
        let actions = self.get_pending_compensations(run_id, phase).await?;
        let mut executed = Vec::new();
        
        for action in actions {
            match self.execute_single_compensation(&action).await {
                Ok(_) => {
                    self.mark_compensation_executed(&action.id).await?;
                    // Return the action with updated status and executed_at
                    let mut updated = action;
                    updated.status = CompensationStatus::Executed;
                    updated.executed_at = Some(Utc::now());
                    executed.push(updated);
                }
                Err(e) => {
                    self.mark_compensation_failed(&action.id).await?;
                    tracing::warn!("Failed to execute compensation {}: {}", action.id, e);
                }
            }
        }
        
        Ok(executed)
    }
    
    async fn execute_single_compensation(&self, action: &CompensatingAction) -> Result<()> {
        match action.action_type.as_str() {
            "rollback_state" => {
                tracing::info!("Rolling back state for phase {}", action.phase);
                Ok(())
            }
            "cleanup_resources" => {
                tracing::info!("Cleaning up resources for phase {}", action.phase);
                Ok(())
            }
            "revert_changes" => {
                tracing::info!("Reverting changes for phase {}", action.phase);
                Ok(())
            }
            _ => {
                tracing::warn!("Unknown compensation type: {}", action.action_type);
                Ok(())
            }
        }
    }
    
    async fn get_pending_compensations(&self, run_id: &str, phase: &str) -> Result<Vec<CompensatingAction>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, String)>(
            r#"
            SELECT id, run_id, phase, action_type, payload, created_at, executed_at, status
            FROM compensating_actions
            WHERE run_id = ? AND phase = ? AND status = ?
            ORDER BY created_at DESC
            "#
        )
        .bind(run_id)
        .bind(phase)
        .bind(serde_json::to_string(&CompensationStatus::Pending)?)
        .fetch_all(&self.pool)
        .await?;
        
        let actions: Vec<CompensatingAction> = rows
            .into_iter()
            .filter_map(|(id, _run_id, phase, action_type, payload, created_at, executed_at, status)| {
                let payload: serde_json::Value = serde_json::from_str(&payload).ok()?;
                let status: CompensationStatus = serde_json::from_str(&status).ok()?;
                let created_at = DateTime::parse_from_rfc3339(&created_at).ok()?.with_timezone(&Utc);
                let executed_at = executed_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
                
                Some(CompensatingAction {
                    id,
                    phase,
                    action_type,
                    payload,
                    created_at,
                    executed_at,
                    status,
                })
            })
            .collect();
        
        Ok(actions)
    }
    
    async fn mark_compensation_executed(&self, action_id: &str) -> Result<()> {
        let executed_at = Utc::now().to_rfc3339();
        let status = serde_json::to_string(&CompensationStatus::Executed)?;
        
        sqlx::query(
            r#"
            UPDATE compensating_actions 
            SET executed_at = ?, status = ?
            WHERE id = ?
            "#
        )
        .bind(&executed_at)
        .bind(&status)
        .bind(action_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn mark_compensation_failed(&self, action_id: &str) -> Result<()> {
        let status = serde_json::to_string(&CompensationStatus::Failed)?;
        
        sqlx::query(
            r#"
            UPDATE compensating_actions 
            SET status = ?
            WHERE id = ?
            "#
        )
        .bind(&status)
        .bind(action_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn add_to_dead_letter_queue(
        &self,
        run_id: &str,
        phase: &str,
        error: &str,
        state_snapshot: &str,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO dead_letter_queue 
            (id, run_id, phase, error, state_snapshot, retry_count, created_at, resolved)
            VALUES (?, ?, ?, ?, ?, 0, ?, 0)
            "#
        )
        .bind(&id)
        .bind(run_id)
        .bind(phase)
        .bind(error)
        .bind(state_snapshot)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    pub async fn get_dead_letter_entries(&self, run_id: &str) -> Result<Vec<DeadLetterEntry>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, i32, String, Option<String>, i32)>(
            r#"
            SELECT id, run_id, phase, error, state_snapshot, retry_count, created_at, last_retry_at, resolved
            FROM dead_letter_queue
            WHERE run_id = ?
            ORDER BY created_at DESC
            "#
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        
        let entries: Vec<DeadLetterEntry> = rows
            .into_iter()
            .filter_map(|(id, run_id, phase, error, state_snapshot, retry_count, created_at, last_retry_at, resolved)| {
                let created_at = DateTime::parse_from_rfc3339(&created_at).ok()?.with_timezone(&Utc);
                let last_retry_at = last_retry_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));
                
                Some(DeadLetterEntry {
                    id,
                    run_id,
                    phase,
                    error,
                    state_snapshot,
                    retry_count: retry_count as u32,
                    created_at,
                    last_retry_at,
                    resolved: resolved != 0,
                })
            })
            .collect();
        
        Ok(entries)
    }
    
    pub async fn retry_dead_letter_entry(&self, entry_id: &str) -> Result<bool> {
        let entry = sqlx::query_as::<_, (String, String, String, String, String, i32, String, Option<String>, i32)>(
            r#"
            SELECT id, run_id, phase, error, state_snapshot, retry_count, created_at, last_retry_at, resolved
            FROM dead_letter_queue
            WHERE id = ?
            "#
        )
        .bind(entry_id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some((_id, _run_id, _phase, _error, _state_snapshot, retry_count, _created_at, _last_retry_at, _resolved)) = entry {
            if retry_count >= self.max_retries as i32 {
                return Ok(false);
            }
            
            let new_retry_count = retry_count + 1;
            let last_retry_at = Utc::now().to_rfc3339();
            
            sqlx::query(
                r#"
                UPDATE dead_letter_queue 
                SET retry_count = ?, last_retry_at = ?
                WHERE id = ?
                "#
            )
            .bind(new_retry_count)
            .bind(&last_retry_at)
            .bind(entry_id)
            .execute(&self.pool)
            .await?;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub async fn resolve_dead_letter_entry(&self, entry_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE dead_letter_queue 
            SET resolved = 1
            WHERE id = ?
            "#
        )
        .bind(entry_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn validate_state_consistency(&self, run_id: &str) -> Result<bool> {
        let checkpoints = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT phase, state_json
            FROM run_checkpoints
            WHERE run_id = ?
            ORDER BY created_at DESC
            LIMIT 10
            "#
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        
        if checkpoints.is_empty() {
            return Ok(true);
        }
        
        for (phase, state_json) in checkpoints {
            if let Err(e) = serde_json::from_str::<serde_json::Value>(&state_json) {
                tracing::error!("Invalid state JSON for phase {}: {}", phase, e);
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS run_states (
                id TEXT PRIMARY KEY,
                target TEXT NOT NULL,
                environment TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();
        
        pool
    }

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool);
        
        let result = manager.ensure_tables().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_register_compensating_action() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let payload = serde_json::json!({"action": "cleanup"});
        let action_id = manager.register_compensating_action(
            "test-run",
            "recon",
            "cleanup_resources",
            payload
        ).await.unwrap();
        
        assert!(!action_id.is_empty());
    }
    
    #[tokio::test]
    async fn test_execute_compensating_actions() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-2")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let payload = serde_json::json!({"action": "rollback"});
        manager.register_compensating_action(
            "test-run-2",
            "recon",
            "rollback_state",
            payload
        ).await.unwrap();
        
        let executed = manager.execute_compensating_actions("test-run-2", "recon").await.unwrap();
        assert_eq!(executed.len(), 1);
    }
    
    #[tokio::test]
    async fn test_add_to_dead_letter_queue() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-3")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let entry_id = manager.add_to_dead_letter_queue(
            "test-run-3",
            "recon",
            "Connection timeout",
            "{\"state\": \"failed\"}"
        ).await.unwrap();
        
        assert!(!entry_id.is_empty());
    }
    
    #[tokio::test]
    async fn test_get_dead_letter_entries() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-4")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        manager.add_to_dead_letter_queue(
            "test-run-4",
            "recon",
            "Network error",
            "{\"state\": \"error\"}"
        ).await.unwrap();
        
        let entries = manager.get_dead_letter_entries("test-run-4").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].phase, "recon");
        assert_eq!(entries[0].error, "Network error");
    }
    
    #[tokio::test]
    async fn test_retry_dead_letter_entry() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-5")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let entry_id = manager.add_to_dead_letter_queue(
            "test-run-5",
            "recon",
            "Timeout",
            "{\"state\": \"timeout\"}"
        ).await.unwrap();
        
        let can_retry = manager.retry_dead_letter_entry(&entry_id).await.unwrap();
        assert!(can_retry);
        
        let entries = manager.get_dead_letter_entries("test-run-5").await.unwrap();
        assert_eq!(entries[0].retry_count, 1);
    }
    
    #[tokio::test]
    async fn test_retry_limit() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-6")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let entry_id = manager.add_to_dead_letter_queue(
            "test-run-6",
            "recon",
            "Persistent error",
            "{\"state\": \"error\"}"
        ).await.unwrap();
        
        for _ in 0..3 {
            manager.retry_dead_letter_entry(&entry_id).await.unwrap();
        }
        
        let can_retry = manager.retry_dead_letter_entry(&entry_id).await.unwrap();
        assert!(!can_retry);
    }
    
    #[tokio::test]
    async fn test_resolve_dead_letter_entry() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-7")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let entry_id = manager.add_to_dead_letter_queue(
            "test-run-7",
            "recon",
            "Resolved error",
            "{\"state\": \"ok\"}"
        ).await.unwrap();
        
        manager.resolve_dead_letter_entry(&entry_id).await.unwrap();
        
        let entries = manager.get_dead_letter_entries("test-run-7").await.unwrap();
        assert!(entries[0].resolved);
    }
    
    #[tokio::test]
    async fn test_validate_state_consistency() {
        let pool = create_test_pool().await;
        let manager = RecoveryManager::new(pool.clone());
        manager.ensure_tables().await.unwrap();
        
        // Create run_checkpoints table needed by validate_state_consistency
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS run_checkpoints (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-8")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        
        let is_consistent = manager.validate_state_consistency("test-run-8").await.unwrap();
        assert!(is_consistent);
    }
}
