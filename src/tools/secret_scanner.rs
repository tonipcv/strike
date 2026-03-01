use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    pub name: String,
    pub pattern: String,
    pub severity: SecretSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecretSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl SecretSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecretSeverity::Critical => "Critical",
            SecretSeverity::High => "High",
            SecretSeverity::Medium => "Medium",
            SecretSeverity::Low => "Low",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub secret_type: String,
    pub value: String,
    pub location: String,
    pub line_number: usize,
    pub severity: SecretSeverity,
    pub confidence: f64,
}

pub struct SecretScanner {
    patterns: Vec<(Regex, SecretPattern)>,
}

impl SecretScanner {
    pub fn new() -> Result<Self> {
        let mut patterns = Vec::new();
        
        for pattern in Self::default_patterns() {
            let regex = Regex::new(&pattern.pattern)?;
            patterns.push((regex, pattern));
        }
        
        Ok(Self { patterns })
    }
    
    fn default_patterns() -> Vec<SecretPattern> {
        vec![
            SecretPattern {
                name: "AWS Access Key".to_string(),
                pattern: r"AKIA[0-9A-Z]{16}".to_string(),
                severity: SecretSeverity::Critical,
                description: "AWS Access Key ID".to_string(),
            },
            SecretPattern {
                name: "AWS Secret Key".to_string(),
                pattern: r#"aws_secret_access_key\s*=\s*['"]?([A-Za-z0-9/+=]{40})['"]?"#.to_string(),
                severity: SecretSeverity::Critical,
                description: "AWS Secret Access Key".to_string(),
            },
            SecretPattern {
                name: "GitHub Token".to_string(),
                pattern: r"ghp_[a-zA-Z0-9]{36}".to_string(),
                severity: SecretSeverity::Critical,
                description: "GitHub Personal Access Token".to_string(),
            },
            SecretPattern {
                name: "GitHub OAuth".to_string(),
                pattern: r"gho_[a-zA-Z0-9]{36}".to_string(),
                severity: SecretSeverity::Critical,
                description: "GitHub OAuth Token".to_string(),
            },
            SecretPattern {
                name: "Slack Token".to_string(),
                pattern: r"xox[baprs]-[0-9]{10,12}-[0-9]{10,12}-[a-zA-Z0-9]{24,32}".to_string(),
                severity: SecretSeverity::High,
                description: "Slack API Token".to_string(),
            },
            SecretPattern {
                name: "Private Key".to_string(),
                pattern: r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----".to_string(),
                severity: SecretSeverity::Critical,
                description: "Private SSH/TLS Key".to_string(),
            },
            SecretPattern {
                name: "Google API Key".to_string(),
                pattern: r"AIza[0-9A-Za-z\\-_]{35}".to_string(),
                severity: SecretSeverity::High,
                description: "Google API Key".to_string(),
            },
            SecretPattern {
                name: "Stripe API Key".to_string(),
                pattern: r"sk_live_[0-9a-zA-Z]{24,}".to_string(),
                severity: SecretSeverity::Critical,
                description: "Stripe Live API Key".to_string(),
            },
            SecretPattern {
                name: "JWT Token".to_string(),
                pattern: r"eyJ[A-Za-z0-9-_=]+\.eyJ[A-Za-z0-9-_=]+\.[A-Za-z0-9-_.+/=]*".to_string(),
                severity: SecretSeverity::Medium,
                description: "JSON Web Token".to_string(),
            },
            SecretPattern {
                name: "Generic API Key".to_string(),
                pattern: r#"api[_-]?key\s*[:=]\s*['"]?([a-zA-Z0-9]{32,})['"]?"#.to_string(),
                severity: SecretSeverity::High,
                description: "Generic API Key".to_string(),
            },
            SecretPattern {
                name: "Generic Password".to_string(),
                pattern: r#"password\s*[:=]\s*['"]([^'"]{8,})['"]"#.to_string(),
                severity: SecretSeverity::Medium,
                description: "Hardcoded Password".to_string(),
            },
            SecretPattern {
                name: "Database Connection String".to_string(),
                pattern: r"(postgres|mysql|mongodb)://[^:]+:[^@]+@".to_string(),
                severity: SecretSeverity::High,
                description: "Database Connection String with Credentials".to_string(),
            },
        ]
    }
    
    pub fn scan_text(&self, text: &str) -> Vec<SecretFinding> {
        let mut findings = Vec::new();
        
        for (line_num, line) in text.lines().enumerate() {
            for (regex, pattern) in &self.patterns {
                if let Some(captures) = regex.captures(line) {
                    let value = captures.get(0).map(|m| m.as_str()).unwrap_or("");
                    
                    findings.push(SecretFinding {
                        secret_type: pattern.name.clone(),
                        value: Self::redact_secret(value),
                        location: format!("line {}", line_num + 1),
                        line_number: line_num + 1,
                        severity: pattern.severity,
                        confidence: Self::calculate_confidence(value, &pattern.name),
                    });
                }
            }
        }
        
        findings
    }
    
    pub fn scan_file(&self, content: &str, filename: &str) -> Vec<SecretFinding> {
        let mut findings = self.scan_text(content);
        
        for finding in &mut findings {
            finding.location = format!("{}:{}", filename, finding.line_number);
        }
        
        findings
    }
    
    pub fn scan_multiple_files(&self, files: HashMap<String, String>) -> HashMap<String, Vec<SecretFinding>> {
        let mut results = HashMap::new();
        
        for (filename, content) in files {
            let findings = self.scan_file(&content, &filename);
            if !findings.is_empty() {
                results.insert(filename, findings);
            }
        }
        
        results
    }
    
    fn redact_secret(value: &str) -> String {
        if value.len() <= 8 {
            "*".repeat(value.len())
        } else {
            format!("{}...{}", &value[..4], &value[value.len()-4..])
        }
    }
    
    fn calculate_confidence(value: &str, pattern_name: &str) -> f64 {
        let mut confidence: f64 = 0.8;
        
        if value.len() > 50 {
            confidence += 0.1;
        }
        
        if pattern_name.contains("AWS") || pattern_name.contains("GitHub") {
            confidence += 0.1;
        }
        
        confidence.min(1.0)
    }
    
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_scanner_creation() {
        let scanner = SecretScanner::new();
        assert!(scanner.is_ok());
    }
    
    #[test]
    fn test_secret_scanner_has_patterns() {
        let scanner = SecretScanner::new().unwrap();
        assert!(scanner.pattern_count() > 0);
    }
    
    #[test]
    fn test_aws_key_detection() {
        let scanner = SecretScanner::new().unwrap();
        let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        
        let findings = scanner.scan_text(text);
        assert!(!findings.is_empty());
    }
    
    #[test]
    fn test_github_token_detection() {
        let scanner = SecretScanner::new().unwrap();
        let text = "token = ghp_1234567890abcdefghijklmnopqrstuv";
        
        let findings = scanner.scan_text(text);
        assert!(!findings.is_empty());
    }
    
    #[test]
    fn test_secret_redaction() {
        let secret = "AKIAIOSFODNN7EXAMPLE";
        let redacted = SecretScanner::redact_secret(secret);
        
        assert!(redacted.contains("AKIA"));
        assert!(redacted.contains("..."));
        assert!(redacted.contains("MPLE"));
    }
    
    #[test]
    fn test_severity_as_str() {
        assert_eq!(SecretSeverity::Critical.as_str(), "Critical");
        assert_eq!(SecretSeverity::High.as_str(), "High");
        assert_eq!(SecretSeverity::Medium.as_str(), "Medium");
        assert_eq!(SecretSeverity::Low.as_str(), "Low");
    }
}
