use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Framework {
    Express,
    Django,
    Rails,
    Laravel,
    SpringBoot,
    Flask,
    FastApi,
    NextJs,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Waf {
    Cloudflare,
    Akamai,
    AwsWaf,
    Imperva,
    F5,
    ModSecurity,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Cdn {
    Cloudflare,
    Akamai,
    Fastly,
    CloudFront,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_header: Option<String>,
    pub powered_by: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    JavaScript,
    Python,
    Ruby,
    Php,
    Java,
    Go,
    Rust,
    Unknown,
}

pub struct Fingerprinter;

impl Fingerprinter {
    pub fn new() -> Self {
        Self
    }
    
    pub fn detect_framework(&self, headers: &HashMap<String, String>, body: &str) -> Vec<Framework> {
        let mut frameworks = Vec::new();
        
        if let Some(powered_by) = headers.get("x-powered-by").or_else(|| headers.get("X-Powered-By")) {
            if powered_by.contains("Express") {
                frameworks.push(Framework::Express);
            } else if powered_by.contains("Django") {
                frameworks.push(Framework::Django);
            } else if powered_by.contains("PHP") {
                frameworks.push(Framework::Laravel);
            }
        }
        
        if body.contains("__next") || body.contains("_next/static") {
            frameworks.push(Framework::NextJs);
        }
        
        if body.contains("csrfmiddlewaretoken") {
            frameworks.push(Framework::Django);
        }
        
        if body.contains("_rails") || headers.values().any(|v| v.contains("Rails")) {
            frameworks.push(Framework::Rails);
        }
        
        if frameworks.is_empty() {
            frameworks.push(Framework::Unknown);
        }
        
        frameworks
    }
    
    pub fn detect_waf(&self, headers: &HashMap<String, String>, _body: &str) -> Option<Waf> {
        if headers.keys().any(|k| k.to_lowercase().contains("cf-ray")) {
            return Some(Waf::Cloudflare);
        }
        
        if headers.keys().any(|k| k.to_lowercase().contains("x-akamai")) {
            return Some(Waf::Akamai);
        }
        
        if headers.keys().any(|k| k.to_lowercase().contains("x-amzn")) {
            return Some(Waf::AwsWaf);
        }
        
        if headers.values().any(|v| v.contains("Imperva")) {
            return Some(Waf::Imperva);
        }
        
        if headers.values().any(|v| v.contains("F5")) {
            return Some(Waf::F5);
        }
        
        None
    }
    
    pub fn detect_cdn(&self, headers: &HashMap<String, String>) -> Option<Cdn> {
        if headers.keys().any(|k| k.to_lowercase().contains("cf-ray")) {
            return Some(Cdn::Cloudflare);
        }
        
        if headers.keys().any(|k| k.to_lowercase().contains("x-akamai")) {
            return Some(Cdn::Akamai);
        }
        
        if headers.keys().any(|k| k.to_lowercase().contains("x-fastly")) {
            return Some(Cdn::Fastly);
        }
        
        if headers.keys().any(|k| k.to_lowercase().contains("x-amz-cf-id")) {
            return Some(Cdn::CloudFront);
        }
        
        None
    }
    
    pub fn detect_server(&self, headers: &HashMap<String, String>) -> ServerInfo {
        let server_header = headers.get("server")
            .or_else(|| headers.get("Server"))
            .cloned();
        
        let powered_by = headers.get("x-powered-by")
            .or_else(|| headers.get("X-Powered-By"))
            .cloned();
        
        let version = server_header.as_ref()
            .and_then(|s| {
                s.split('/')
                    .nth(1)
                    .map(|v| v.split_whitespace().next().unwrap_or("").to_string())
            });
        
        ServerInfo {
            server_header,
            powered_by,
            version,
        }
    }
    
    pub fn detect_language(&self, headers: &HashMap<String, String>, body: &str) -> Option<Language> {
        if let Some(powered_by) = headers.get("x-powered-by").or_else(|| headers.get("X-Powered-By")) {
            if powered_by.contains("Express") || powered_by.contains("Node") {
                return Some(Language::JavaScript);
            } else if powered_by.contains("PHP") {
                return Some(Language::Php);
            }
        }
        
        if body.contains("__next") || body.contains("_next/static") {
            return Some(Language::JavaScript);
        }
        
        if body.contains("csrfmiddlewaretoken") || body.contains("Django") {
            return Some(Language::Python);
        }
        
        if body.contains("_rails") {
            return Some(Language::Ruby);
        }
        
        if headers.values().any(|v| v.contains("Spring")) {
            return Some(Language::Java);
        }
        
        None
    }
}

impl Default for Fingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprinter_creation() {
        let fingerprinter = Fingerprinter::new();
        assert_eq!(fingerprinter, Fingerprinter);
    }
    
    #[test]
    fn test_detect_framework_express() {
        let fingerprinter = Fingerprinter::new();
        let mut headers = HashMap::new();
        headers.insert("X-Powered-By".to_string(), "Express".to_string());
        
        let frameworks = fingerprinter.detect_framework(&headers, "");
        assert!(frameworks.contains(&Framework::Express));
    }
    
    #[test]
    fn test_detect_framework_nextjs() {
        let fingerprinter = Fingerprinter::new();
        let headers = HashMap::new();
        let body = "<script src=\"/_next/static/chunks/main.js\"></script>";
        
        let frameworks = fingerprinter.detect_framework(&headers, body);
        assert!(frameworks.contains(&Framework::NextJs));
    }
    
    #[test]
    fn test_detect_framework_django() {
        let fingerprinter = Fingerprinter::new();
        let headers = HashMap::new();
        let body = "<input type='hidden' name='csrfmiddlewaretoken' value='abc123'>";
        
        let frameworks = fingerprinter.detect_framework(&headers, body);
        assert!(frameworks.contains(&Framework::Django));
    }
    
    #[test]
    fn test_detect_waf_cloudflare() {
        let fingerprinter = Fingerprinter::new();
        let mut headers = HashMap::new();
        headers.insert("cf-ray".to_string(), "12345".to_string());
        
        let waf = fingerprinter.detect_waf(&headers, "");
        assert_eq!(waf, Some(Waf::Cloudflare));
    }
    
    #[test]
    fn test_detect_cdn_cloudflare() {
        let fingerprinter = Fingerprinter::new();
        let mut headers = HashMap::new();
        headers.insert("cf-ray".to_string(), "12345".to_string());
        
        let cdn = fingerprinter.detect_cdn(&headers);
        assert_eq!(cdn, Some(Cdn::Cloudflare));
    }
    
    #[test]
    fn test_detect_server() {
        let fingerprinter = Fingerprinter::new();
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "nginx/1.18.0".to_string());
        headers.insert("X-Powered-By".to_string(), "PHP/7.4".to_string());
        
        let server_info = fingerprinter.detect_server(&headers);
        assert_eq!(server_info.server_header, Some("nginx/1.18.0".to_string()));
        assert_eq!(server_info.powered_by, Some("PHP/7.4".to_string()));
        assert_eq!(server_info.version, Some("1.18.0".to_string()));
    }
    
    #[test]
    fn test_detect_language_javascript() {
        let fingerprinter = Fingerprinter::new();
        let mut headers = HashMap::new();
        headers.insert("X-Powered-By".to_string(), "Express".to_string());
        
        let language = fingerprinter.detect_language(&headers, "");
        assert_eq!(language, Some(Language::JavaScript));
    }
    
    #[test]
    fn test_detect_language_python() {
        let fingerprinter = Fingerprinter::new();
        let headers = HashMap::new();
        let body = "<input name='csrfmiddlewaretoken'>";
        
        let language = fingerprinter.detect_language(&headers, body);
        assert_eq!(language, Some(Language::Python));
    }
}
