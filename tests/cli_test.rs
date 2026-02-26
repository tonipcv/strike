use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Evidence-first CLI security validation platform"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("recon"));
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_init_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "init", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialize a new engagement workspace"));
    assert!(stdout.contains("--target"));
    assert!(stdout.contains("--env"));
}

#[test]
fn test_run_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Execute a full validation pipeline"));
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("--workers"));
}

#[test]
fn test_recon_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "recon", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Standalone reconnaissance phase"));
    assert!(stdout.contains("--target"));
    assert!(stdout.contains("--subdomains"));
}

#[test]
fn test_findings_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "findings", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Query and filter findings"));
    assert!(stdout.contains("--severity"));
    assert!(stdout.contains("--status"));
}
