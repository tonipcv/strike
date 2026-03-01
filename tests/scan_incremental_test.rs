use strike_security::ci::incremental::{IncrementalScanner, ScanDiff, ChangeType};
use std::collections::HashSet;

#[test]
fn test_incremental_scanner_creation() {
    let scanner = IncrementalScanner::new();
    assert!(scanner.is_ok());
}

#[test]
fn test_scan_diff_empty() {
    let diff = ScanDiff::new();
    
    assert!(diff.added_endpoints.is_empty());
    assert!(diff.modified_endpoints.is_empty());
    assert!(diff.removed_endpoints.is_empty());
    assert_eq!(diff.total_changes(), 0);
}

#[test]
fn test_scan_diff_with_changes() {
    let mut diff = ScanDiff::new();
    
    diff.add_endpoint("/api/users".to_string());
    diff.modify_endpoint("/api/posts".to_string());
    diff.remove_endpoint("/api/legacy".to_string());
    
    assert_eq!(diff.added_endpoints.len(), 1);
    assert_eq!(diff.modified_endpoints.len(), 1);
    assert_eq!(diff.removed_endpoints.len(), 1);
    assert_eq!(diff.total_changes(), 3);
}

#[test]
fn test_change_type_variants() {
    let added = ChangeType::Added;
    let modified = ChangeType::Modified;
    let removed = ChangeType::Removed;
    let unchanged = ChangeType::Unchanged;
    
    assert!(matches!(added, ChangeType::Added));
    assert!(matches!(modified, ChangeType::Modified));
    assert!(matches!(removed, ChangeType::Removed));
    assert!(matches!(unchanged, ChangeType::Unchanged));
}

#[tokio::test]
async fn test_incremental_scanner_compare() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let old_endpoints = vec![
        "/api/users".to_string(),
        "/api/posts".to_string(),
        "/api/comments".to_string(),
    ];
    
    let new_endpoints = vec![
        "/api/users".to_string(),
        "/api/posts/v2".to_string(),
        "/api/tags".to_string(),
    ];
    
    let diff = scanner.compare_endpoints(&old_endpoints, &new_endpoints);
    
    assert!(diff.added_endpoints.contains("/api/tags"));
    assert!(diff.removed_endpoints.contains("/api/comments"));
}

#[tokio::test]
async fn test_incremental_scanner_no_changes() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let endpoints = vec![
        "/api/users".to_string(),
        "/api/posts".to_string(),
    ];
    
    let diff = scanner.compare_endpoints(&endpoints, &endpoints);
    
    assert_eq!(diff.total_changes(), 0);
}

#[tokio::test]
async fn test_incremental_scanner_all_new() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let old_endpoints: Vec<String> = vec![];
    let new_endpoints = vec![
        "/api/users".to_string(),
        "/api/posts".to_string(),
    ];
    
    let diff = scanner.compare_endpoints(&old_endpoints, &new_endpoints);
    
    assert_eq!(diff.added_endpoints.len(), 2);
    assert_eq!(diff.removed_endpoints.len(), 0);
}

#[tokio::test]
async fn test_incremental_scanner_all_removed() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let old_endpoints = vec![
        "/api/users".to_string(),
        "/api/posts".to_string(),
    ];
    let new_endpoints: Vec<String> = vec![];
    
    let diff = scanner.compare_endpoints(&old_endpoints, &new_endpoints);
    
    assert_eq!(diff.added_endpoints.len(), 0);
    assert_eq!(diff.removed_endpoints.len(), 2);
}

#[test]
fn test_scan_diff_deduplication() {
    let mut diff = ScanDiff::new();
    
    diff.add_endpoint("/api/users".to_string());
    diff.add_endpoint("/api/users".to_string());
    
    assert_eq!(diff.added_endpoints.len(), 1);
}

#[tokio::test]
async fn test_incremental_scanner_priority_scoring() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let score_new = scanner.calculate_priority_score(ChangeType::Added);
    let score_modified = scanner.calculate_priority_score(ChangeType::Modified);
    let score_removed = scanner.calculate_priority_score(ChangeType::Removed);
    let score_unchanged = scanner.calculate_priority_score(ChangeType::Unchanged);
    
    assert!(score_new > score_modified);
    assert!(score_modified > score_unchanged);
    assert_eq!(score_removed, 0);
}

#[tokio::test]
async fn test_incremental_scanner_should_scan() {
    let scanner = IncrementalScanner::new().unwrap();
    
    assert!(scanner.should_scan(ChangeType::Added));
    assert!(scanner.should_scan(ChangeType::Modified));
    assert!(!scanner.should_scan(ChangeType::Unchanged));
    assert!(!scanner.should_scan(ChangeType::Removed));
}

#[test]
fn test_scan_diff_merge() {
    let mut diff1 = ScanDiff::new();
    diff1.add_endpoint("/api/users".to_string());
    
    let mut diff2 = ScanDiff::new();
    diff2.add_endpoint("/api/posts".to_string());
    
    let merged = diff1.merge(diff2);
    
    assert_eq!(merged.added_endpoints.len(), 2);
}

#[tokio::test]
async fn test_incremental_scanner_batch_processing() {
    let scanner = IncrementalScanner::new().unwrap();
    
    let endpoints = (0..100)
        .map(|i| format!("/api/endpoint{}", i))
        .collect::<Vec<_>>();
    
    let batches = scanner.create_batches(&endpoints, 10);
    
    assert_eq!(batches.len(), 10);
    assert_eq!(batches[0].len(), 10);
}

#[test]
fn test_change_type_serialization() {
    let change = ChangeType::Added;
    let json = serde_json::to_string(&change).unwrap();
    let deserialized: ChangeType = serde_json::from_str(&json).unwrap();
    
    assert_eq!(change, deserialized);
}
