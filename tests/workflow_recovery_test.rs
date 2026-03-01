use strike_security::workflow::recovery::{RecoveryManager, CompensationStatus};
use sqlx::SqlitePool;
use chrono::Utc;

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
async fn test_recovery_manager_tables() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool);
    
    let result = manager.ensure_tables().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_compensating_action_registration() {
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
    
    let payload = serde_json::json!({"action": "cleanup", "resource": "temp_files"});
    let action_id = manager.register_compensating_action(
        "test-run",
        "recon",
        "cleanup_resources",
        payload
    ).await.unwrap();
    
    assert!(!action_id.is_empty());
}

#[tokio::test]
async fn test_compensating_action_execution() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-exec")
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
        "test-run-exec",
        "recon",
        "rollback_state",
        payload
    ).await.unwrap();
    
    let executed = manager.execute_compensating_actions("test-run-exec", "recon").await.unwrap();
    assert_eq!(executed.len(), 1);
    assert_eq!(executed[0].status, CompensationStatus::Executed);
}

#[tokio::test]
async fn test_dead_letter_queue_add() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-dlq")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    let entry_id = manager.add_to_dead_letter_queue(
        "test-run-dlq",
        "recon",
        "Connection timeout",
        "{\"state\": \"failed\"}"
    ).await.unwrap();
    
    assert!(!entry_id.is_empty());
}

#[tokio::test]
async fn test_dead_letter_queue_retrieval() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-dlq-get")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    manager.add_to_dead_letter_queue(
        "test-run-dlq-get",
        "recon",
        "Network error",
        "{\"state\": \"error\"}"
    ).await.unwrap();
    
    let entries = manager.get_dead_letter_entries("test-run-dlq-get").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].phase, "recon");
    assert_eq!(entries[0].error, "Network error");
    assert_eq!(entries[0].retry_count, 0);
    assert!(!entries[0].resolved);
}

#[tokio::test]
async fn test_dead_letter_queue_retry() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-retry")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    let entry_id = manager.add_to_dead_letter_queue(
        "test-run-retry",
        "recon",
        "Timeout",
        "{\"state\": \"timeout\"}"
    ).await.unwrap();
    
    let can_retry = manager.retry_dead_letter_entry(&entry_id).await.unwrap();
    assert!(can_retry);
    
    let entries = manager.get_dead_letter_entries("test-run-retry").await.unwrap();
    assert_eq!(entries[0].retry_count, 1);
    assert!(entries[0].last_retry_at.is_some());
}

#[tokio::test]
async fn test_dead_letter_queue_max_retries() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-max-retry")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    let entry_id = manager.add_to_dead_letter_queue(
        "test-run-max-retry",
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
async fn test_dead_letter_queue_resolution() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-resolve")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    let entry_id = manager.add_to_dead_letter_queue(
        "test-run-resolve",
        "recon",
        "Resolved error",
        "{\"state\": \"ok\"}"
    ).await.unwrap();
    
    manager.resolve_dead_letter_entry(&entry_id).await.unwrap();
    
    let entries = manager.get_dead_letter_entries("test-run-resolve").await.unwrap();
    assert!(entries[0].resolved);
}

#[tokio::test]
async fn test_state_consistency_validation() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-consistency")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    let is_consistent = manager.validate_state_consistency("test-run-consistency").await.unwrap();
    assert!(is_consistent);
}

#[tokio::test]
async fn test_multiple_compensating_actions() {
    let pool = create_test_pool().await;
    let manager = RecoveryManager::new(pool.clone());
    manager.ensure_tables().await.unwrap();
    
    sqlx::query("INSERT INTO run_states (id, target, environment, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind("test-run-multi")
        .bind("https://example.com")
        .bind("test")
        .bind("running")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    
    for i in 0..5 {
        let payload = serde_json::json!({"action": format!("cleanup_{}", i)});
        manager.register_compensating_action(
            "test-run-multi",
            "recon",
            "cleanup_resources",
            payload
        ).await.unwrap();
    }
    
    let executed = manager.execute_compensating_actions("test-run-multi", "recon").await.unwrap();
    assert_eq!(executed.len(), 5);
}
