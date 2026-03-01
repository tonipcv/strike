use strike_security::config::validation::InputValidator;

#[test]
fn test_ssrf_localhost_blocking() {
    assert!(InputValidator::validate_url("http://localhost/api", false).is_err());
    assert!(InputValidator::validate_url("http://127.0.0.1/api", false).is_err());
    assert!(InputValidator::validate_url("http://[::1]/api", false).is_err());
    assert!(InputValidator::validate_url("http://0.0.0.0/api", false).is_err());
}

#[test]
fn test_ssrf_private_ip_blocking() {
    assert!(InputValidator::validate_url("http://10.0.0.1/api", false).is_err());
    assert!(InputValidator::validate_url("http://172.16.0.1/api", false).is_err());
    assert!(InputValidator::validate_url("http://192.168.1.1/api", false).is_err());
    assert!(InputValidator::validate_url("http://169.254.169.254/metadata", false).is_err());
}

#[test]
fn test_ssrf_internal_tld_blocking() {
    assert!(InputValidator::validate_url("http://internal.local/api", false).is_err());
    assert!(InputValidator::validate_url("http://service.internal/api", false).is_err());
    assert!(InputValidator::validate_url("http://app.corp/api", false).is_err());
    assert!(InputValidator::validate_url("http://server.private/api", false).is_err());
}

#[test]
fn test_ssrf_metadata_service_blocking() {
    assert!(InputValidator::validate_url("http://169.254.169.254/latest/meta-data", false).is_err());
    assert!(InputValidator::validate_url("http://metadata.google.internal/", false).is_err());
}

#[test]
fn test_ssrf_allow_private_flag() {
    assert!(InputValidator::validate_url("http://localhost/api", true).is_ok());
    assert!(InputValidator::validate_url("http://127.0.0.1/api", true).is_ok());
    assert!(InputValidator::validate_url("http://192.168.1.1/api", true).is_ok());
}

#[test]
fn test_ssrf_public_urls_allowed() {
    assert!(InputValidator::validate_url("https://example.com/api", false).is_ok());
    assert!(InputValidator::validate_url("https://api.github.com/users", false).is_ok());
    assert!(InputValidator::validate_url("https://www.google.com/search", false).is_ok());
}

#[test]
fn test_path_traversal_prevention() {
    assert!(InputValidator::validate_file_path("../etc/passwd").is_err());
    assert!(InputValidator::validate_file_path("../../windows/system32").is_err());
    assert!(InputValidator::validate_file_path("./../../secrets").is_err());
    assert!(InputValidator::validate_file_path("reports/../../../etc/passwd").is_err());
}

#[test]
fn test_path_traversal_safe_paths() {
    assert!(InputValidator::validate_file_path("reports/2024/january.pdf").is_ok());
    assert!(InputValidator::validate_file_path("data/findings.json").is_ok());
    assert!(InputValidator::validate_file_path("output.txt").is_ok());
}

#[test]
fn test_null_byte_injection_prevention() {
    assert!(InputValidator::validate_string("test\0.txt").is_err());
    assert!(InputValidator::validate_string("file%00.pdf").is_err());
    assert!(InputValidator::validate_string("data\x00injection").is_err());
}

#[test]
fn test_null_byte_safe_strings() {
    assert!(InputValidator::validate_string("normal_file.txt").is_ok());
    assert!(InputValidator::validate_string("report-2024.pdf").is_ok());
    assert!(InputValidator::validate_string("user@example.com").is_ok());
}

#[test]
fn test_header_injection_prevention() {
    assert!(InputValidator::sanitize_header_value("test\r\nX-Injected: value").is_err());
    assert!(InputValidator::sanitize_header_value("value\nSet-Cookie: session=abc").is_err());
    assert!(InputValidator::sanitize_header_value("test\rinjection").is_err());
}

#[test]
fn test_header_injection_safe_values() {
    assert!(InputValidator::sanitize_header_value("application/json").is_ok());
    assert!(InputValidator::sanitize_header_value("Bearer token123").is_ok());
    assert!(InputValidator::sanitize_header_value("en-US,en;q=0.9").is_ok());
}

#[test]
fn test_rate_limit_validation() {
    assert!(InputValidator::validate_rate_limit(0).is_err());
    assert!(InputValidator::validate_rate_limit(10).is_ok());
    assert!(InputValidator::validate_rate_limit(100).is_ok());
    assert!(InputValidator::validate_rate_limit(1001).is_err());
}

#[test]
fn test_timeout_validation() {
    assert!(InputValidator::validate_timeout(0).is_err());
    assert!(InputValidator::validate_timeout(30).is_ok());
    assert!(InputValidator::validate_timeout(300).is_ok());
    assert!(InputValidator::validate_timeout(3601).is_err());
}

#[test]
fn test_worker_count_validation() {
    assert!(InputValidator::validate_worker_count(0).is_err());
    assert!(InputValidator::validate_worker_count(4).is_ok());
    assert!(InputValidator::validate_worker_count(32).is_ok());
    assert!(InputValidator::validate_worker_count(129).is_err());
}

#[test]
fn test_url_scheme_validation() {
    assert!(InputValidator::validate_url("https://example.com", false).is_ok());
    assert!(InputValidator::validate_url("http://example.com", false).is_ok());
    assert!(InputValidator::validate_url("ftp://example.com", false).is_err());
    assert!(InputValidator::validate_url("file:///etc/passwd", false).is_err());
}

#[test]
fn test_combined_ssrf_attacks() {
    assert!(InputValidator::validate_url("http://127.0.0.1@example.com", false).is_err());
    assert!(InputValidator::validate_url("http://example.com@127.0.0.1", false).is_err());
    assert!(InputValidator::validate_url("http://localhost.example.com", false).is_err());
}

#[test]
fn test_ipv6_localhost_blocking() {
    assert!(InputValidator::validate_url("http://[::1]/api", false).is_err());
    assert!(InputValidator::validate_url("http://[0:0:0:0:0:0:0:1]/api", false).is_err());
}

#[test]
fn test_link_local_blocking() {
    assert!(InputValidator::validate_url("http://169.254.1.1/api", false).is_err());
    assert!(InputValidator::validate_url("http://169.254.255.255/api", false).is_err());
}

#[test]
fn test_multiple_validation_layers() {
    let url = "https://example.com/api/users";
    let path = "reports/findings.json";
    let header = "application/json";
    
    assert!(InputValidator::validate_url(url, false).is_ok());
    assert!(InputValidator::validate_file_path(path).is_ok());
    assert!(InputValidator::sanitize_header_value(header).is_ok());
}

#[test]
fn test_edge_case_empty_strings() {
    assert!(InputValidator::validate_string("").is_ok());
    assert!(InputValidator::validate_file_path("").is_err());
}

#[test]
fn test_unicode_in_validation() {
    assert!(InputValidator::validate_string("测试文件.txt").is_ok());
    assert!(InputValidator::validate_string("файл.pdf").is_ok());
    assert!(InputValidator::sanitize_header_value("UTF-8 ✓").is_ok());
}

#[test]
fn test_very_long_inputs() {
    let long_string = "a".repeat(10000);
    assert!(InputValidator::validate_string(&long_string).is_ok());
    
    let long_path = format!("{}/file.txt", "dir/".repeat(100));
    assert!(InputValidator::validate_file_path(&long_path).is_ok());
}

#[test]
fn test_special_characters_in_paths() {
    assert!(InputValidator::validate_file_path("file with spaces.txt").is_ok());
    assert!(InputValidator::validate_file_path("file-with-dashes.txt").is_ok());
    assert!(InputValidator::validate_file_path("file_with_underscores.txt").is_ok());
}

#[test]
fn test_url_with_query_parameters() {
    assert!(InputValidator::validate_url("https://example.com/api?param=value", false).is_ok());
    assert!(InputValidator::validate_url("https://example.com/search?q=test&page=1", false).is_ok());
}

#[test]
fn test_url_with_fragments() {
    assert!(InputValidator::validate_url("https://example.com/page#section", false).is_ok());
    assert!(InputValidator::validate_url("https://example.com/docs#api-reference", false).is_ok());
}

#[test]
fn test_url_with_authentication() {
    assert!(InputValidator::validate_url("https://user:pass@example.com/api", false).is_ok());
}

#[test]
fn test_case_sensitivity_in_validation() {
    assert!(InputValidator::validate_url("http://LOCALHOST/api", false).is_err());
    assert!(InputValidator::validate_url("http://LocalHost/api", false).is_err());
}
