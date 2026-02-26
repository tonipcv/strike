use strike_security::workflow::{
    CheckpointManager, EventBus, PhaseConfig, WorkflowEngine, WorkflowEvent, WorkflowPhase,
    WorkflowState,
};
use sqlx::SqlitePool;
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
        "#,
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
async fn test_checkpoint_save_and_restore() {
    let pool = create_test_pool().await;
    let manager = CheckpointManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();

    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-run")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();

    let state = WorkflowState::new("test-run".to_string());
    let checkpoint_id = manager
        .save_checkpoint("test-run", "recon", &state)
        .await
        .unwrap();

    assert!(!checkpoint_id.is_empty());

    let restored = manager.restore_state("test-run").await.unwrap();
    assert!(restored.is_some());
}

#[tokio::test]
async fn test_resume_from_checkpoint() {
    let pool = create_test_pool().await;

    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-resume")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();

    {
        let engine = WorkflowEngine::new("test-resume".to_string(), pool.clone(), None)
            .await
            .unwrap();

        engine
            .start_phase(WorkflowPhase::ScopeValidation)
            .await
            .unwrap();
        engine
            .complete_phase(WorkflowPhase::ScopeValidation, None, 500)
            .await
            .unwrap();
    }

    let resumed = WorkflowEngine::resume("test-resume".to_string(), pool, None)
        .await
        .unwrap();

    let state = resumed.get_state().await;
    assert!(state.is_phase_completed("SCOPE_VALIDATION"));
}

#[tokio::test]
async fn test_event_bus() {
    let bus = EventBus::new(100);
    let mut receiver = bus.subscribe();

    let event = WorkflowEvent::RunStarted {
        run_id: "test-run".to_string(),
        timestamp: chrono::Utc::now(),
    };

    bus.publish(event.clone());

    let received = receiver.recv().await.unwrap();
    assert_eq!(received.run_id(), "test-run");
}

#[tokio::test]
async fn test_phase_transitions_are_idempotent() {
    let pool = create_test_pool().await;
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-idempotent")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();
    
    let engine = WorkflowEngine::new("test-idempotent".to_string(), pool, None)
        .await
        .unwrap();

    engine
        .start_phase(WorkflowPhase::Recon)
        .await
        .unwrap();
    engine
        .complete_phase(WorkflowPhase::Recon, None, 1000)
        .await
        .unwrap();

    engine
        .start_phase(WorkflowPhase::Recon)
        .await
        .unwrap();
    engine
        .complete_phase(WorkflowPhase::Recon, None, 1000)
        .await
        .unwrap();

    let state = engine.get_state().await;
    assert!(state.is_phase_completed("RECON"));
}

#[tokio::test]
async fn test_workflow_phases_configuration() {
    let pipeline = PhaseConfig::default_pipeline();
    assert_eq!(pipeline.len(), 10);

    let scope_phase = &pipeline[0];
    assert_eq!(scope_phase.phase, WorkflowPhase::ScopeValidation);
    assert!(scope_phase.blocking);
    assert!(!scope_phase.parallel);

    let validation_phase = pipeline
        .iter()
        .find(|p| p.phase == WorkflowPhase::Validation)
        .unwrap();
    assert!(validation_phase.parallel);
    assert!(!validation_phase.blocking);
}

#[tokio::test]
async fn test_get_next_phases_respects_dependencies() {
    let pool = create_test_pool().await;
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-deps")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();
    
    let engine = WorkflowEngine::new("test-deps".to_string(), pool, None)
        .await
        .unwrap();

    let next = engine.get_next_phases().await.unwrap();
    assert!(next.contains(&WorkflowPhase::ScopeValidation));
    assert!(!next.contains(&WorkflowPhase::Recon));

    engine
        .start_phase(WorkflowPhase::ScopeValidation)
        .await
        .unwrap();
    engine
        .complete_phase(WorkflowPhase::ScopeValidation, None, 500)
        .await
        .unwrap();

    let next = engine.get_next_phases().await.unwrap();
    assert!(next.contains(&WorkflowPhase::Recon));
    assert!(next.contains(&WorkflowPhase::AuthBootstrap));
    assert!(!next.contains(&WorkflowPhase::HypothesisGeneration));
}

#[tokio::test]
async fn test_checkpoint_saved_on_phase_completion() {
    let pool = create_test_pool().await;

    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-checkpoint")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();

    let engine = WorkflowEngine::new("test-checkpoint".to_string(), pool.clone(), None)
        .await
        .unwrap();

    engine
        .start_phase(WorkflowPhase::Recon)
        .await
        .unwrap();
    engine
        .complete_phase(WorkflowPhase::Recon, None, 1000)
        .await
        .unwrap();

    let manager = CheckpointManager::new(pool);
    let checkpoint = manager
        .load_latest_checkpoint("test-checkpoint")
        .await
        .unwrap();

    assert!(checkpoint.is_some());
    let cp = checkpoint.unwrap();
    assert_eq!(cp.phase, "RECON");
}

#[tokio::test]
async fn test_event_log_stored_correctly() {
    let pool = create_test_pool().await;
    let manager = CheckpointManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();

    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))")
        .bind("test-events")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .execute(&pool)
        .await
        .unwrap();

    let event_payload = serde_json::json!({
        "phase": "recon",
        "status": "completed"
    });

    manager
        .save_event("test-events", "PhaseCompleted", &event_payload)
        .await
        .unwrap();

    let events = manager.get_events("test-events").await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].1, "PhaseCompleted");
}
