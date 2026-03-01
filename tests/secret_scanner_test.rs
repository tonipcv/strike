use strike_security::tools::secret_scanner::{SecretScanner, SecretSeverity};
use std::collections::HashMap;

#[test]
fn test_secret_scanner_creation() {
    let scanner = SecretScanner::new();
    assert!(scanner.is_ok());
}

#[test]
fn test_secret_scanner_default() {
    let scanner = SecretScanner::default();
    assert!(scanner.pattern_count() > 0);
}

#[test]
fn test_secret_scanner_has_multiple_patterns() {
    let scanner = SecretScanner::new().unwrap();
    assert!(scanner.pattern_count() >= 10);
}

#[test]
fn test_aws_access_key_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    
    let findings = scanner.scan_text(text);
    
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].secret_type, "AWS Access Key");
    assert_eq!(findings[0].severity, SecretSeverity::Critical);
}

#[test]
fn test_github_token_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "export GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuv";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.secret_type.contains("GitHub")));
}

#[test]
fn test_private_key_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert_eq!(findings[0].secret_type, "Private Key");
    assert_eq!(findings[0].severity, SecretSeverity::Critical);
}

#[test]
fn test_slack_token_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "SLACK_TOKEN=xoxb-1234567890-1234567890-abcdefghijklmnopqrstuvwx";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
}

#[test]
fn test_jwt_token_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.secret_type == "JWT Token"));
}

#[test]
fn test_google_api_key_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "GOOGLE_API_KEY=AIzaSyDaGmWKa4JsXZ-HjGw7ISLn_3namBGewQe";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
}

#[test]
fn test_stripe_api_key_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "STRIPE_KEY=sk_live_4eC39HqLyjWDarjtT1zdp7dc";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert_eq!(findings[0].severity, SecretSeverity::Critical);
}

#[test]
fn test_database_connection_string_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "DATABASE_URL=postgres://user:password@localhost:5432/db";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
}

#[test]
fn test_generic_api_key_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "api_key = \"1234567890abcdef1234567890abcdef\"";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
}

#[test]
fn test_hardcoded_password_detection() {
    let scanner = SecretScanner::new().unwrap();
    let text = "password = \"MySecretPassword123\"";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
}

#[test]
fn test_no_secrets_in_clean_text() {
    let scanner = SecretScanner::new().unwrap();
    let text = "This is a clean text with no secrets";
    
    let findings = scanner.scan_text(text);
    
    assert!(findings.is_empty());
}

#[test]
fn test_secret_redaction() {
    let scanner = SecretScanner::new().unwrap();
    let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert!(findings[0].value.contains("..."));
    assert!(!findings[0].value.contains("EXAMPLE"));
}

#[test]
fn test_line_number_tracking() {
    let scanner = SecretScanner::new().unwrap();
    let text = "line 1\nline 2\nAWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\nline 4";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert_eq!(findings[0].line_number, 3);
}

#[test]
fn test_scan_file_with_filename() {
    let scanner = SecretScanner::new().unwrap();
    let content = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    
    let findings = scanner.scan_file(content, "config.env");
    
    assert!(!findings.is_empty());
    assert!(findings[0].location.contains("config.env"));
}

#[test]
fn test_scan_multiple_files() {
    let scanner = SecretScanner::new().unwrap();
    
    let mut files = HashMap::new();
    files.insert("file1.env".to_string(), "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE".to_string());
    files.insert("file2.env".to_string(), "GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuv".to_string());
    files.insert("file3.txt".to_string(), "No secrets here".to_string());
    
    let results = scanner.scan_multiple_files(files);
    
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("file1.env"));
    assert!(results.contains_key("file2.env"));
    assert!(!results.contains_key("file3.txt"));
}

#[test]
fn test_confidence_score() {
    let scanner = SecretScanner::new().unwrap();
    let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    
    let findings = scanner.scan_text(text);
    
    assert!(!findings.is_empty());
    assert!(findings[0].confidence > 0.0);
    assert!(findings[0].confidence <= 1.0);
}

#[test]
fn test_severity_levels() {
    assert_eq!(SecretSeverity::Critical.as_str(), "Critical");
    assert_eq!(SecretSeverity::High.as_str(), "High");
    assert_eq!(SecretSeverity::Medium.as_str(), "Medium");
    assert_eq!(SecretSeverity::Low.as_str(), "Low");
}

#[test]
fn test_multiple_secrets_in_same_file() {
    let scanner = SecretScanner::new().unwrap();
    let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\nGITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuv";
    
    let findings = scanner.scan_text(text);
    
    assert!(findings.len() >= 2);
}
