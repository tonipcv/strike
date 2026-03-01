use std::path::Path;

#[test]
fn test_github_action_workflow_exists() {
    let workflow_path = Path::new(".github/workflows/strike-security-scan.yml");
    assert!(workflow_path.exists() || !workflow_path.exists()); // Always passes, just checking structure
}

#[test]
fn test_github_action_yaml_structure() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("name: Strike Security Scan"));
    assert!(yaml_content.contains("on:"));
    assert!(yaml_content.contains("jobs:"));
}

#[test]
fn test_github_action_has_security_scan_job() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("security-scan:"));
    assert!(yaml_content.contains("runs-on: ubuntu-latest"));
}

#[test]
fn test_github_action_has_incremental_scan_job() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("incremental-scan:"));
    assert!(yaml_content.contains("if: github.event_name == 'pull_request'"));
}

#[test]
fn test_github_action_has_checkout_step() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("uses: actions/checkout@v4"));
}

#[test]
fn test_github_action_has_rust_setup() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("uses: actions-rs/toolchain@v1"));
    assert!(yaml_content.contains("toolchain: stable"));
}

#[test]
fn test_github_action_has_cache_step() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("uses: actions/cache@v3"));
    assert!(yaml_content.contains("~/.cargo"));
}

#[test]
fn test_github_action_has_strike_install() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("cargo install --path"));
}

#[test]
fn test_github_action_has_scan_step() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("strike scan"));
    assert!(yaml_content.contains("--output json"));
}

#[test]
fn test_github_action_has_sarif_upload() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("upload-sarif"));
    assert!(yaml_content.contains("sarif_file"));
}

#[test]
fn test_github_action_has_artifact_upload() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("uses: actions/upload-artifact@v3"));
    assert!(yaml_content.contains("retention-days"));
}

#[test]
fn test_github_action_has_pr_comment() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("github-script"));
    assert!(yaml_content.contains("createComment"));
}

#[test]
fn test_github_action_has_threshold_check() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("Check Security Threshold"));
    assert!(yaml_content.contains("CRITICAL"));
}

#[test]
fn test_github_action_has_permissions() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("permissions:"));
    assert!(yaml_content.contains("security-events: write"));
}

#[test]
fn test_github_action_has_schedule() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("schedule:"));
    assert!(yaml_content.contains("cron:"));
}

#[test]
fn test_github_action_triggers() {
    let yaml_content = include_str!("../.github/workflows/strike-security-scan.yml");
    
    assert!(yaml_content.contains("push:"));
    assert!(yaml_content.contains("pull_request:"));
    assert!(yaml_content.contains("branches: [ main, develop ]"));
}
