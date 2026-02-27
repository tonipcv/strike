use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTarget {
    pub name: String,
    pub url: String,
    pub expected_vulns: Vec<ExpectedVuln>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedVuln {
    pub vuln_class: String,
    pub endpoint: String,
    pub parameter: Option<String>,
    pub description: String,
}

pub struct TargetRegistry;

impl TargetRegistry {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get_juice_shop() -> BenchmarkTarget {
        BenchmarkTarget {
            name: "OWASP Juice Shop".to_string(),
            url: "http://localhost:3000".to_string(),
            expected_vulns: vec![
                ExpectedVuln {
                    vuln_class: "SQLi".to_string(),
                    endpoint: "/rest/user/login".to_string(),
                    parameter: Some("email".to_string()),
                    description: "SQL Injection in login".to_string(),
                },
                ExpectedVuln {
                    vuln_class: "XSS".to_string(),
                    endpoint: "/".to_string(),
                    parameter: Some("q".to_string()),
                    description: "Reflected XSS in search".to_string(),
                },
                ExpectedVuln {
                    vuln_class: "IDOR".to_string(),
                    endpoint: "/api/Users/:id".to_string(),
                    parameter: Some("id".to_string()),
                    description: "IDOR in user profile".to_string(),
                },
                ExpectedVuln {
                    vuln_class: "BOLA".to_string(),
                    endpoint: "/api/Baskets/:id".to_string(),
                    parameter: Some("id".to_string()),
                    description: "BOLA in basket access".to_string(),
                },
            ],
        }
    }
    
    pub fn get_webgoat() -> BenchmarkTarget {
        BenchmarkTarget {
            name: "WebGoat".to_string(),
            url: "http://localhost:8080/WebGoat".to_string(),
            expected_vulns: vec![
                ExpectedVuln {
                    vuln_class: "SQLi".to_string(),
                    endpoint: "/SqlInjection/attack1".to_string(),
                    parameter: Some("account_name".to_string()),
                    description: "SQL Injection intro".to_string(),
                },
                ExpectedVuln {
                    vuln_class: "AuthBypass".to_string(),
                    endpoint: "/auth-bypass/1".to_string(),
                    parameter: None,
                    description: "Authentication bypass".to_string(),
                },
            ],
        }
    }
    
    pub fn get_dvwa() -> BenchmarkTarget {
        BenchmarkTarget {
            name: "DVWA".to_string(),
            url: "http://localhost:80/DVWA".to_string(),
            expected_vulns: vec![
                ExpectedVuln {
                    vuln_class: "SQLi".to_string(),
                    endpoint: "/vulnerabilities/sqli/".to_string(),
                    parameter: Some("id".to_string()),
                    description: "SQL Injection".to_string(),
                },
                ExpectedVuln {
                    vuln_class: "XSS".to_string(),
                    endpoint: "/vulnerabilities/xss_r/".to_string(),
                    parameter: Some("name".to_string()),
                    description: "Reflected XSS".to_string(),
                },
            ],
        }
    }
    
    pub fn get_all_targets() -> Vec<BenchmarkTarget> {
        vec![
            Self::get_juice_shop(),
            Self::get_webgoat(),
            Self::get_dvwa(),
        ]
    }
}

impl Default for TargetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_registry_creation() {
        let registry = TargetRegistry::new();
        assert_eq!(registry, TargetRegistry);
    }
    
    #[test]
    fn test_get_juice_shop() {
        let target = TargetRegistry::get_juice_shop();
        
        assert_eq!(target.name, "OWASP Juice Shop");
        assert!(!target.expected_vulns.is_empty());
        assert!(target.expected_vulns.iter().any(|v| v.vuln_class == "SQLi"));
    }
    
    #[test]
    fn test_get_webgoat() {
        let target = TargetRegistry::get_webgoat();
        
        assert_eq!(target.name, "WebGoat");
        assert!(!target.expected_vulns.is_empty());
    }
    
    #[test]
    fn test_get_dvwa() {
        let target = TargetRegistry::get_dvwa();
        
        assert_eq!(target.name, "DVWA");
        assert!(!target.expected_vulns.is_empty());
    }
    
    #[test]
    fn test_get_all_targets() {
        let targets = TargetRegistry::get_all_targets();
        
        assert_eq!(targets.len(), 3);
        assert!(targets.iter().any(|t| t.name == "OWASP Juice Shop"));
        assert!(targets.iter().any(|t| t.name == "WebGoat"));
        assert!(targets.iter().any(|t| t.name == "DVWA"));
    }
}
