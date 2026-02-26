use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::state::WorkflowState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub run_id: String,
    pub phase: String,
    pub state_json: String,
    pub created_at: DateTime<Utc>,
    pub metadata: String,
}

pub struct CheckpointManager {
    pool: SqlitePool,
}

impl CheckpointManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    pub async fn ensure_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS run_checkpoints (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT NOT NULL,
                FOREIGN KEY (run_id) REFERENCES run_states(id)
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_events (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (run_id) REFERENCES run_states(id)
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_checkpoints_run_id 
            ON run_checkpoints(run_id)
            "#
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_events_run_id 
            ON workflow_events(run_id)
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn save_checkpoint(
        &self,
        run_id: &str,
        phase: &str,
        state: &WorkflowState,
    ) -> Result<String> {
        let checkpoint_id = Uuid::new_v4().to_string();
        let state_json = serde_json::to_string(state)?;
        let created_at = Utc::now().to_rfc3339();
        let metadata = "{}";
        
        sqlx::query(
            r#"
            INSERT INTO run_checkpoints (id, run_id, phase, state_json, created_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&checkpoint_id)
        .bind(run_id)
        .bind(phase)
        .bind(&state_json)
        .bind(&created_at)
        .bind(metadata)
        .execute(&self.pool)
        .await?;
        
        Ok(checkpoint_id)
    }
    
    pub async fn load_latest_checkpoint(&self, run_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoint = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            r#"
            SELECT id, run_id, phase, state_json, created_at, metadata
            FROM run_checkpoints
            WHERE run_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(checkpoint.map(|(id, run_id, phase, state_json, created_at, metadata)| {
            Checkpoint {
                id,
                run_id,
                phase,
                state_json,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .unwrap()
                    .with_timezone(&Utc),
                metadata,
            }
        }))
    }
    
    pub async fn restore_state(&self, run_id: &str) -> Result<Option<WorkflowState>> {
        if let Some(checkpoint) = self.load_latest_checkpoint(run_id).await? {
            let state: WorkflowState = serde_json::from_str(&checkpoint.state_json)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
    
    pub async fn save_event(
        &self,
        run_id: &str,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<String> {
        let event_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(payload)?;
        let created_at = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO workflow_events (id, run_id, event_type, payload_json, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#
        )
        .bind(&event_id)
        .bind(run_id)
        .bind(event_type)
        .bind(&payload_json)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;
        
        Ok(event_id)
    }
    
    pub async fn get_events(&self, run_id: &str) -> Result<Vec<(String, String, serde_json::Value, DateTime<Utc>)>> {
        let events = sqlx::query_as::<_, (String, String, String, String)>(
            r#"
            SELECT id, event_type, payload_json, created_at
            FROM workflow_events
            WHERE run_id = ?
            ORDER BY created_at ASC
            "#
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        
        let parsed_events: Vec<_> = events
            .into_iter()
            .filter_map(|(id, event_type, payload_json, created_at)| {
                let payload: serde_json::Value = serde_json::from_str(&payload_json).ok()?;
                let timestamp = DateTime::parse_from_rfc3339(&created_at)
                    .ok()?
                    .with_timezone(&Utc);
                Some((id, event_type, payload, timestamp))
            })
            .collect();
        
        Ok(parsed_events)
    }
    
    pub async fn delete_checkpoints(&self, run_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM run_checkpoints WHERE run_id = ?")
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_pool() -> SqlitePool {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        
        SqlitePool::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_checkpoint_manager_creation() {
        let pool = create_test_pool().await;
        let manager = CheckpointManager::new(pool);
        
        let result = manager.ensure_tables().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_save_and_load_checkpoint() {
        let pool = create_test_pool().await;
        let manager = CheckpointManager::new(pool);
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&manager.pool)
            .await
            .unwrap();
        
        let state = WorkflowState::new("test-run".to_string());
        
        let checkpoint_id = manager.save_checkpoint("test-run", "recon", &state).await.unwrap();
        assert!(!checkpoint_id.is_empty());
        
        let loaded = manager.load_latest_checkpoint("test-run").await.unwrap();
        assert!(loaded.is_some());
        
        let checkpoint = loaded.unwrap();
        assert_eq!(checkpoint.run_id, "test-run");
        assert_eq!(checkpoint.phase, "recon");
    }
    
    #[tokio::test]
    async fn test_restore_state() {
        let pool = create_test_pool().await;
        let manager = CheckpointManager::new(pool);
        manager.ensure_tables().await.unwrap();
        
        sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind("test-run-2")
            .bind("https://example.com")
            .bind("test")
            .bind("running")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&manager.pool)
            .await
            .unwrap();
        
        let mut state = WorkflowState::new("test-run-2".to_string());
        state.start_phase("recon".to_string());
        state.complete_phase("recon".to_string(), None);
        
        manager.save_checkpoint("test-run-2", "recon", &state).await.unwrap();
        
        let restored = manager.restore_state("test-run-2").await.unwrap();
        assert!(restored.is_some());
        
        let restored_state = restored.unwrap();
        assert_eq!(restored_state.run_id, "test-run-2");
        assert!(restored_state.is_phase_completed("recon"));
    }
}
