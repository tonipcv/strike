use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::checkpoint::CheckpointManager;
use super::events::{EventBus, WorkflowEvent};
use super::phases::{PhaseConfig, WorkflowPhase};
use super::state::{PhaseStatus, WorkflowState};

pub struct WorkflowEngine {
    state: Arc<RwLock<WorkflowState>>,
    checkpoint_manager: Arc<CheckpointManager>,
    event_bus: EventBus,
    pipeline: Vec<PhaseConfig>,
    auto_checkpoint: bool,
}

impl WorkflowEngine {
    pub async fn new(
        run_id: String,
        pool: SqlitePool,
        pipeline: Option<Vec<PhaseConfig>>,
    ) -> Result<Self> {
        let checkpoint_manager = Arc::new(CheckpointManager::new(pool));
        checkpoint_manager.ensure_tables().await?;
        
        let state = Arc::new(RwLock::new(WorkflowState::new(run_id.clone())));
        let event_bus = EventBus::new(1000);
        let pipeline = pipeline.unwrap_or_else(PhaseConfig::default_pipeline);
        
        let engine = Self {
            state,
            checkpoint_manager,
            event_bus,
            pipeline,
            auto_checkpoint: true,
        };
        
        engine.publish_event(WorkflowEvent::RunStarted {
            run_id,
            timestamp: Utc::now(),
        });
        
        Ok(engine)
    }
    
    pub async fn resume(
        run_id: String,
        pool: SqlitePool,
        pipeline: Option<Vec<PhaseConfig>>,
    ) -> Result<Self> {
        let checkpoint_manager = Arc::new(CheckpointManager::new(pool));
        checkpoint_manager.ensure_tables().await?;
        
        let restored_state = checkpoint_manager
            .restore_state(&run_id)
            .await?
            .context("No checkpoint found for run")?;
        
        let state = Arc::new(RwLock::new(restored_state));
        let event_bus = EventBus::new(1000);
        let pipeline = pipeline.unwrap_or_else(PhaseConfig::default_pipeline);
        
        Ok(Self {
            state,
            checkpoint_manager,
            event_bus,
            pipeline,
            auto_checkpoint: true,
        })
    }
    
    pub async fn start_phase(&self, phase: WorkflowPhase) -> Result<()> {
        let phase_name = phase.name().to_string();
        
        {
            let mut state = self.state.write().await;
            state.start_phase(phase_name.clone());
        }
        
        self.publish_event(WorkflowEvent::PhaseStarted {
            run_id: self.get_run_id().await,
            phase: phase_name.clone(),
            timestamp: Utc::now(),
        });
        
        if self.auto_checkpoint {
            self.save_checkpoint(&phase_name).await?;
        }
        
        Ok(())
    }
    
    pub async fn complete_phase(
        &self,
        phase: WorkflowPhase,
        output: Option<serde_json::Value>,
        duration_ms: u64,
    ) -> Result<()> {
        let phase_name = phase.name().to_string();
        
        {
            let mut state = self.state.write().await;
            state.complete_phase(phase_name.clone(), output);
        }
        
        self.publish_event(WorkflowEvent::PhaseCompleted {
            run_id: self.get_run_id().await,
            phase: phase_name.clone(),
            duration_ms,
            timestamp: Utc::now(),
        });
        
        if self.auto_checkpoint {
            self.save_checkpoint(&phase_name).await?;
        }
        
        Ok(())
    }
    
    pub async fn fail_phase(&self, phase: WorkflowPhase, error: String) -> Result<()> {
        let phase_name = phase.name().to_string();
        
        {
            let mut state = self.state.write().await;
            state.fail_phase(phase_name.clone(), error.clone());
        }
        
        self.publish_event(WorkflowEvent::PhaseFailed {
            run_id: self.get_run_id().await,
            phase: phase_name.clone(),
            error,
            timestamp: Utc::now(),
        });
        
        if self.auto_checkpoint {
            self.save_checkpoint(&phase_name).await?;
        }
        
        Ok(())
    }
    
    pub async fn save_checkpoint(&self, phase: &str) -> Result<String> {
        let state = self.state.read().await;
        let run_id = state.run_id.clone();
        
        let checkpoint_id = self
            .checkpoint_manager
            .save_checkpoint(&run_id, phase, &state)
            .await?;
        
        self.publish_event(WorkflowEvent::CheckpointSaved {
            run_id,
            checkpoint_id: checkpoint_id.clone(),
            phase: phase.to_string(),
            timestamp: Utc::now(),
        });
        
        Ok(checkpoint_id)
    }
    
    pub async fn get_next_phases(&self) -> Result<Vec<WorkflowPhase>> {
        let state = self.state.read().await;
        let completed_phases = state.get_completed_phases();
        
        let mut next_phases = Vec::new();
        
        for phase_config in &self.pipeline {
            let phase_name = phase_config.phase.name();
            
            if state.is_phase_completed(phase_name) || state.is_phase_failed(phase_name) {
                continue;
            }
            
            let dependencies_met = phase_config
                .dependencies
                .iter()
                .all(|dep| state.is_phase_completed(dep.name()));
            
            if dependencies_met {
                next_phases.push(phase_config.phase.clone());
            }
        }
        
        Ok(next_phases)
    }
    
    pub async fn is_phase_ready(&self, phase: &WorkflowPhase) -> Result<bool> {
        let state = self.state.read().await;
        
        let phase_config = self
            .pipeline
            .iter()
            .find(|p| &p.phase == phase)
            .context("Phase not found in pipeline")?;
        
        let dependencies_met = phase_config
            .dependencies
            .iter()
            .all(|dep| state.is_phase_completed(dep.name()));
        
        Ok(dependencies_met)
    }
    
    pub async fn complete_run(&self, total_findings: usize, duration_ms: u64) -> Result<()> {
        {
            let mut state = self.state.write().await;
            state.complete_run();
        }
        
        self.publish_event(WorkflowEvent::RunCompleted {
            run_id: self.get_run_id().await,
            total_findings,
            duration_ms,
            timestamp: Utc::now(),
        });
        
        Ok(())
    }
    
    pub async fn fail_run(&self, error: String) -> Result<()> {
        self.publish_event(WorkflowEvent::RunFailed {
            run_id: self.get_run_id().await,
            error,
            timestamp: Utc::now(),
        });
        
        Ok(())
    }
    
    pub fn publish_event(&self, event: WorkflowEvent) {
        self.event_bus.publish(event);
    }
    
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<WorkflowEvent> {
        self.event_bus.subscribe()
    }
    
    pub async fn get_run_id(&self) -> String {
        let state = self.state.read().await;
        state.run_id.clone()
    }
    
    pub async fn get_state(&self) -> WorkflowState {
        let state = self.state.read().await;
        state.clone()
    }
    
    pub async fn get_phase_status(&self, phase: &WorkflowPhase) -> Option<PhaseStatus> {
        let state = self.state.read().await;
        state.phase_states.get(phase.name()).map(|s| s.status.clone())
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
        
        let pool = SqlitePool::connect(&db_url).await.unwrap();
        
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
    async fn test_workflow_engine_creation() {
        let pool = create_test_pool().await;
        let engine = WorkflowEngine::new("test-run".to_string(), pool, None).await;
        assert!(engine.is_ok());
    }
    
    #[tokio::test]
    async fn test_phase_lifecycle() {
        let pool = create_test_pool().await;
        let engine = WorkflowEngine::new("test-run".to_string(), pool, None).await.unwrap();
        
        engine.start_phase(WorkflowPhase::Recon).await.unwrap();
        
        let status = engine.get_phase_status(&WorkflowPhase::Recon).await;
        assert_eq!(status, Some(PhaseStatus::Running));
        
        engine.complete_phase(WorkflowPhase::Recon, None, 1000).await.unwrap();
        
        let status = engine.get_phase_status(&WorkflowPhase::Recon).await;
        assert_eq!(status, Some(PhaseStatus::Completed));
    }
    
    #[tokio::test]
    async fn test_checkpoint_and_resume() {
        let pool = create_test_pool().await;
        
        {
            let engine = WorkflowEngine::new("test-run-resume".to_string(), pool.clone(), None).await.unwrap();
            
            sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind("test-run-resume")
                .bind("https://example.com")
                .bind("test")
                .bind("running")
                .bind(Utc::now().to_rfc3339())
                .bind(Utc::now().to_rfc3339())
                .execute(&pool)
                .await
                .unwrap();
            
            engine.start_phase(WorkflowPhase::Recon).await.unwrap();
            engine.complete_phase(WorkflowPhase::Recon, None, 1000).await.unwrap();
        }
        
        let resumed_engine = WorkflowEngine::resume("test-run-resume".to_string(), pool, None).await.unwrap();
        
        let state = resumed_engine.get_state().await;
        assert!(state.is_phase_completed("RECON"));
    }
    
    #[tokio::test]
    async fn test_get_next_phases() {
        let pool = create_test_pool().await;
        let engine = WorkflowEngine::new("test-run-next".to_string(), pool, None).await.unwrap();
        
        let next = engine.get_next_phases().await.unwrap();
        assert!(next.contains(&WorkflowPhase::ScopeValidation));
        
        engine.start_phase(WorkflowPhase::ScopeValidation).await.unwrap();
        engine.complete_phase(WorkflowPhase::ScopeValidation, None, 500).await.unwrap();
        
        let next = engine.get_next_phases().await.unwrap();
        assert!(next.contains(&WorkflowPhase::Recon));
        assert!(next.contains(&WorkflowPhase::AuthBootstrap));
    }
}
