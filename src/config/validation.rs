use anyhow::{bail, Context, Result};
use std::net::IpAddr;
use url::Url;

pub struct InputValidator;

impl InputValidator {
    pub fn validate_url(url_str: &str, allow_private: bool) -> Result<Url> {
        // Check for @ in URL which could be used for SSRF bypass via credentials
        if url_str.contains('@') {
            let url = Url::parse(url_str).context("Invalid URL format")?;
            // Check if the username part contains suspicious IPs
            let username = url.username();
            if !username.is_empty() {
                // Check if username looks like an IP address or localhost
                if username.parse::<IpAddr>().is_ok() || username.eq_ignore_ascii_case("localhost") {
                    bail!("URL contains suspicious credentials that look like an IP address - potential SSRF bypass");
                }
            }
        }
        
        let url = Url::parse(url_str)
            .context("Invalid URL format")?;
        
        if url.scheme() != "http" && url.scheme() != "https" {
            bail!("Only HTTP and HTTPS schemes are allowed, got: {}", url.scheme());
        }
        
        let host = url.host_str()
            .context("URL must have a valid host")?;
        
        if !allow_private {
            Self::check_ssrf_blocklist(host)?;
        }
        
        Ok(url)
    }
    
    pub fn validate_target_url(url_str: &str) -> Result<Url> {
        Self::validate_url(url_str, false)
    }
    
    pub fn validate_local_url(url_str: &str) -> Result<Url> {
        Self::validate_url(url_str, true)
    }
    
    fn check_ssrf_blocklist(host: &str) -> Result<()> {
        // Check for @ in host which could be used for SSRF bypass
        if host.contains('@') {
            bail!("Host cannot contain '@' character - potential SSRF bypass attempt");
        }
        
        let host_lower = host.to_ascii_lowercase();
        if host_lower == "localhost" || host_lower.ends_with(".localhost") || host_lower.contains("localhost.") {
            bail!("Localhost targets are blocked for security. Use --allow-private if this is intentional.");
        }
        
        // Remove brackets from IPv6 addresses if present
        let host_clean = host.trim_start_matches('[').trim_end_matches(']');
        
        if let Ok(ip) = host_clean.parse::<IpAddr>() {
            if Self::is_private_ip(&ip) {
                bail!("Private IP addresses are blocked for security: {}. Use --allow-private if this is intentional.", ip);
            }
            
            if Self::is_link_local(&ip) {
                bail!("Link-local addresses are blocked for security: {}", ip);
            }
            
            if Self::is_loopback(&ip) {
                bail!("Loopback addresses are blocked for security: {}", ip);
            }
            
            if Self::is_metadata_service(&ip) {
                bail!("Cloud metadata service addresses are blocked for security: {}", ip);
            }
        }
        
        if Self::is_internal_hostname(host) {
            bail!("Internal hostnames are blocked for security: {}. Use --allow-private if this is intentional.", host);
        }
        
        Ok(())
    }
    
    fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_private() || 
                ipv4.octets()[0] == 10 ||
                (ipv4.octets()[0] == 172 && (ipv4.octets()[1] >= 16 && ipv4.octets()[1] <= 31)) ||
                (ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168)
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_unique_local() || 
                ipv6.segments()[0] & 0xfe00 == 0xfc00
            }
        }
    }
    
    fn is_link_local(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254,
            IpAddr::V6(ipv6) => ipv6.segments()[0] & 0xffc0 == 0xfe80,
        }
    }
    
    fn is_loopback(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_loopback() || ipv4.is_unspecified(),
            IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unspecified(),
        }
    }
    
    fn is_metadata_service(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.octets() == [169, 254, 169, 254]
            }
            IpAddr::V6(ipv6) => {
                ipv6.segments() == [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0]
            }
        }
    }
    
    fn is_internal_hostname(host: &str) -> bool {
        let internal_tlds = [
            ".internal", ".local", ".lan", ".corp", ".home", ".intranet",
            ".private", ".localdomain"
        ];
        
        for tld in &internal_tlds {
            if host.ends_with(tld) {
                return true;
            }
        }
        
        false
    }
    
    pub fn validate_file_path(path: &str) -> Result<String> {
        if path.is_empty() {
            bail!("File path cannot be empty");
        }
        
        if path.contains("..") {
            bail!("Path traversal detected: path cannot contain '..'");
        }
        
        if path.starts_with('/') && !path.starts_with("/tmp/") && !path.starts_with("/var/tmp/") {
            bail!("Absolute paths outside /tmp are not allowed for security");
        }
        
        if cfg!(windows) && path.contains("\\..\\") {
            bail!("Path traversal detected: path cannot contain '..\\' on Windows");
        }
        
        let null_byte_patterns = ["\0", "%00", "\\x00"];
        for pattern in &null_byte_patterns {
            if path.contains(pattern) {
                bail!("Null byte injection detected in path");
            }
        }
        
        Ok(path.to_string())
    }
    
    pub fn validate_string(value: &str) -> Result<String> {
        // Reuse the same null byte detection patterns used in file path validation
        let null_byte_patterns = ["\0", "%00", "\\x00"];
        for pattern in &null_byte_patterns {
            if value.contains(pattern) {
                bail!("Null byte injection detected in string");
            }
        }
        Ok(value.to_string())
    }
    
    pub fn sanitize_header_value(value: &str) -> Result<String> {
        if value.contains('\n') || value.contains('\r') {
            bail!("Header injection detected: value cannot contain newlines");
        }
        
        if value.len() > 8192 {
            bail!("Header value too long (max 8192 bytes)");
        }
        
        Ok(value.to_string())
    }
    
    pub fn validate_rate_limit(rate: u32) -> Result<u32> {
        if rate == 0 {
            bail!("Rate limit must be greater than 0");
        }
        
        if rate > 1000 {
            bail!("Rate limit too high (max 1000 requests/second)");
        }
        
        Ok(rate)
    }
    
    pub fn validate_timeout(timeout: u32) -> Result<u32> {
        if timeout == 0 {
            bail!("Timeout must be greater than 0");
        }
        
        if timeout > 3600 {
            bail!("Timeout too high (max 3600 seconds)");
        }
        
        Ok(timeout)
    }
    
    pub fn validate_worker_count(workers: u32) -> Result<u32> {
        if workers == 0 {
            bail!("Worker count must be greater than 0");
        }
        
        if workers > 128 {
            bail!("Worker count too high (max 128)");
        }
        
        Ok(workers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_public_url() {
        let result = InputValidator::validate_target_url("https://example.com");
        assert!(result.is_ok());
        
        let result = InputValidator::validate_target_url("http://api.github.com/repos");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_block_localhost() {
        let result = InputValidator::validate_target_url("http://localhost:8080");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Localhost"));
        
        let result = InputValidator::validate_target_url("http://api.localhost");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_block_loopback_ip() {
        let result = InputValidator::validate_target_url("http://127.0.0.1");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Loopback") || err_msg.contains("loopback"));
        
        let result = InputValidator::validate_target_url("http://[::1]:8080");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_block_private_ips() {
        let private_ips = [
            "http://10.0.0.1",
            "http://172.16.0.1",
            "http://192.168.1.1",
            "http://192.168.0.100:8080",
        ];
        
        for ip in &private_ips {
            let result = InputValidator::validate_target_url(ip);
            assert!(result.is_err(), "Should block {}", ip);
            assert!(result.unwrap_err().to_string().contains("Private IP"));
        }
    }
    
    #[test]
    fn test_block_link_local() {
        let result = InputValidator::validate_target_url("http://169.254.1.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Link-local"));
    }
    
    #[test]
    fn test_block_metadata_service() {
        let result = InputValidator::validate_target_url("http://169.254.169.254");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        // 169.254.169.254 is both link-local and metadata service, either error is acceptable
        assert!(err_msg.contains("metadata") || err_msg.contains("Cloud metadata") || err_msg.contains("Link-local"));
    }
    
    #[test]
    fn test_block_internal_hostnames() {
        let internal_hosts = [
            "http://api.internal",
            "http://server.local",
            "http://app.corp",
            "http://service.private",
        ];
        
        for host in &internal_hosts {
            let result = InputValidator::validate_target_url(host);
            assert!(result.is_err(), "Should block {}", host);
            assert!(result.unwrap_err().to_string().contains("Internal hostname"));
        }
    }
    
    #[test]
    fn test_allow_private_with_flag() {
        let result = InputValidator::validate_local_url("http://localhost:8080");
        assert!(result.is_ok());
        
        let result = InputValidator::validate_local_url("http://192.168.1.1");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_block_non_http_schemes() {
        let schemes = [
            "file:///etc/passwd",
            "ftp://example.com",
            "gopher://example.com",
            "data:text/html,<script>alert(1)</script>",
        ];
        
        for scheme in &schemes {
            let result = InputValidator::validate_target_url(scheme);
            assert!(result.is_err(), "Should block {}", scheme);
            assert!(result.unwrap_err().to_string().contains("HTTP"));
        }
    }
    
    #[test]
    fn test_validate_file_path_traversal() {
        let result = InputValidator::validate_file_path("../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
        
        let result = InputValidator::validate_file_path("config/../../secrets");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_file_path_null_byte() {
        let result = InputValidator::validate_file_path("file.txt\0.jpg");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Null byte"));
        
        let result = InputValidator::validate_file_path("file%00.txt");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_file_path_safe() {
        let result = InputValidator::validate_file_path("config.yaml");
        assert!(result.is_ok());
        
        let result = InputValidator::validate_file_path("/tmp/strike-output.json");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_sanitize_header_injection() {
        let result = InputValidator::sanitize_header_value("normal-value");
        assert!(result.is_ok());
        
        let result = InputValidator::sanitize_header_value("value\ninjected: header");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("injection"));
        
        let result = InputValidator::sanitize_header_value("value\r\ninjected: header");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_rate_limit() {
        assert!(InputValidator::validate_rate_limit(50).is_ok());
        assert!(InputValidator::validate_rate_limit(0).is_err());
        assert!(InputValidator::validate_rate_limit(10001).is_err());
    }
    
    #[test]
    fn test_validate_timeout() {
        assert!(InputValidator::validate_timeout(30).is_ok());
        assert!(InputValidator::validate_timeout(0).is_err());
        assert!(InputValidator::validate_timeout(3601).is_err());
    }
    
    #[test]
    fn test_validate_worker_count() {
        assert!(InputValidator::validate_worker_count(16).is_ok());
        assert!(InputValidator::validate_worker_count(0).is_err());
        assert!(InputValidator::validate_worker_count(257).is_err());
    }
}
