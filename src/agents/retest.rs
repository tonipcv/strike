use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::DateTime;

use crate::models::finding::Finding;
use crate::tools::http_client::HttpClient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RetestStatus {
    Fixed,
    StillVulnerable,
    CannotReproduce,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestResult {
    pub finding_id: String,
    pub status: RetestStatus,
    pub verified_at: chrono::DateTime<chrono::Utc>,
    pub response_diff: Option<ResponseDiff>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiff {
    pub baseline_status: u16,
    pub retest_status: u16,
    pub status_changed: bool,
    pub body_changed: bool,
}

pub struct RetestAgent {
    http_client: HttpClient,
}

impl RetestAgent {
    pub fn new() -> Self {
        Self {
            http_client: HttpClient::new(50, 30).unwrap(),
        }
    }
    
    pub async fn retest_finding(&self, finding: &Finding) -> Result<RetestResult> {
        let expect_fixed = false;
        let status = if expect_fixed {
            self.verify_fix(finding).await?
        } else {
            self.verify_still_vulnerable(finding).await?
        };
        
        Ok(RetestResult {
            finding_id: finding.id.to_string(),
            status,
            verified_at: chrono::Utc::now(),
            response_diff: None,
            notes: "Automated retest completed".to_string(),
        })
    }
    
    async fn verify_fix(&self, finding: &Finding) -> Result<RetestStatus> {
        // Re-execute the original payload and compare response
        let target_url = &finding.target.url;
        
        // Extract payload from finding description or use common test payloads
        let test_payloads = vec![
            "' OR '1'='1",
            "<script>alert(1)</script>",
            "http://169.254.169.254/",
            "../../../etc/passwd",
        ];
        
        let mut vulnerability_detected = false;
        
        for payload in test_payloads {
            let test_url = format!("{}?test={}", target_url, payload);
            
            match self.http_client.get(&test_url).await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    
                    // Check for vulnerability indicators
                    let has_sql_error = body.contains("SQL") || body.contains("syntax") || body.contains("mysql");
                    let has_xss = body.contains("<script>") || body.contains(payload);
                    let has_ssrf = body.contains("169.254") || body.contains("metadata");
                    let has_lfi = body.contains("root:") || body.contains("/etc/passwd");
                    
                    if has_sql_error || has_xss || has_ssrf || has_lfi || status.is_server_error() {
                        vulnerability_detected = true;
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
        
        if vulnerability_detected {
            Ok(RetestStatus::StillVulnerable)
        } else {
            Ok(RetestStatus::Fixed)
        }
    }
    
    async fn verify_still_vulnerable(&self, finding: &Finding) -> Result<RetestStatus> {
        let is_vulnerable = self.verify_still_vulnerable_real(finding).await?;
        
        if is_vulnerable {
            Ok(RetestStatus::StillVulnerable)
        } else {
            Ok(RetestStatus::CannotReproduce)
        }
    }
    
    async fn verify_still_vulnerable_real(&self, finding: &Finding) -> Result<bool> {
        // Re-execute and check if still exploitable
        let target_url = &finding.target.url;
        
        // Use vulnerability-specific payloads based on vuln_class
        let payloads = match finding.vuln_class {
            crate::models::VulnClass::SqlInjection => vec![
                "' OR '1'='1",
                "' OR 1=1--",
                "admin'--",
                "' UNION SELECT NULL--",
            ],
            crate::models::VulnClass::XssReflected | crate::models::VulnClass::XssStored => vec![
                "<script>alert(1)</script>",
                "<img src=x onerror=alert(1)>",
                "javascript:alert(1)",
            ],
            crate::models::VulnClass::Ssrf => vec![
                "http://169.254.169.254/",
                "http://metadata.google.internal/",
                "http://localhost:8080",
            ],
            _ => vec!["test_payload"],
        };
        
        for payload in payloads {
            let test_url = format!("{}?input={}", target_url, payload);
            
            match self.http_client.get(&test_url).await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    
                    // Check for vulnerability indicators based on vuln_class
                    let is_vulnerable = match finding.vuln_class {
                        crate::models::VulnClass::SqlInjection => {
                            body.contains("SQL") || body.contains("syntax") || body.contains("mysql") || status.is_server_error()
                        },
                        crate::models::VulnClass::XssReflected | crate::models::VulnClass::XssStored => {
                            body.contains("<script>") || body.contains(payload)
                        },
                        crate::models::VulnClass::Ssrf => {
                            body.contains("169.254") || body.contains("metadata") || body.len() > 1000
                        },
                        _ => false,
                    };
                    
                    if is_vulnerable {
                        return Ok(true);
                    }
                }
                Err(_) => continue,
            }
        }
        
        Ok(false)
    }
    
    pub async fn bulk_retest(&self, findings: &[Finding]) -> Vec<RetestResult> {
        let mut results = Vec::new();
        
        for finding in findings {
            match self.retest_finding(finding).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(RetestResult {
                        finding_id: finding.id.to_string(),
                        status: RetestStatus::Error,
                        verified_at: chrono::Utc::now(),
                        response_diff: None,
                        notes: format!("Error: {}", e),
                    });
                }
            }
        }
        
        results
    }
    
    pub fn calculate_fix_rate(&self, results: &[RetestResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }
        
        let fixed_count = results.iter()
            .filter(|r| r.status == RetestStatus::Fixed)
            .count();
        
        (fixed_count as f32 / results.len() as f32) * 100.0
    }
    
    pub fn generate_closure_report(&self, results: &[RetestResult]) -> ClosureReport {
        let total = results.len();
        let fixed = results.iter().filter(|r| r.status == RetestStatus::Fixed).count();
        let still_vulnerable = results.iter().filter(|r| r.status == RetestStatus::StillVulnerable).count();
        let cannot_reproduce = results.iter().filter(|r| r.status == RetestStatus::CannotReproduce).count();
        let errors = results.iter().filter(|r| r.status == RetestStatus::Error).count();
        
        ClosureReport {
            total,
            fixed,
            still_vulnerable,
            cannot_reproduce,
            errors,
            fix_rate: self.calculate_fix_rate(results),
        }
    }
}

impl Default for RetestAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureReport {
    pub total: usize,
    pub fixed: usize,
    pub still_vulnerable: usize,
    pub cannot_reproduce: usize,
    pub errors: usize,
    pub fix_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::finding::{Target, HttpMethod};
    use crate::models::vuln_class::VulnClass;
    use crate::models::severity::Severity;

    fn create_test_finding() -> Finding {
        Finding {
            id: "test-finding-1".to_string(),
            run_id: "test-run".to_string(),
            title: "Test Finding".to_string(),
            vuln_class: VulnClass::Idor,
            severity: Severity::High,
            target: Target {
                url: "https://example.com/api/users/123".to_string(),
                method: HttpMethod::Get,
                parameter: Some("id".to_string()),
            },
            confidence: 0.9,
            cvss_score: 7.5,
            owasp_category: "A01:2021".to_string(),
            cwe_id: "CWE-639".to_string(),
            description: "Test description".to_string(),
            impact: "Test impact".to_string(),
            remediation: "Test remediation".to_string(),
            references: vec![],
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_retest_agent_creation() {
        let agent = RetestAgent::new();
        assert_eq!(agent, RetestAgent::default());
    }
    
    #[tokio::test]
    async fn test_retest_finding_expect_fixed() {
        let agent = RetestAgent::new();
        let finding = create_test_finding();
        
        let result = agent.retest_finding(&finding, true).await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert_eq!(result.status, RetestStatus::Fixed);
    }
    
    #[tokio::test]
    async fn test_retest_finding_expect_vulnerable() {
        let agent = RetestAgent::new();
        let finding = create_test_finding();
        
        let result = agent.retest_finding(&finding, false).await;
        assert!(result.is_ok());
        
        let result = result.unwrap();
        assert_eq!(result.status, RetestStatus::StillVulnerable);
    }
    
    #[test]
    fn test_calculate_fix_rate() {
        let agent = RetestAgent::new();
        
        let results = vec![
            RetestResult {
                finding_id: "1".to_string(),
                status: RetestStatus::Fixed,
                verified_at: chrono::Utc::now(),
                response_diff: None,
                notes: "".to_string(),
            },
            RetestResult {
                finding_id: "2".to_string(),
                status: RetestStatus::Fixed,
                verified_at: chrono::Utc::now(),
                response_diff: None,
                notes: "".to_string(),
            },
            RetestResult {
                finding_id: "3".to_string(),
                status: RetestStatus::StillVulnerable,
                verified_at: chrono::Utc::now(),
                response_diff: None,
                notes: "".to_string(),
            },
        ];
        
        let fix_rate = agent.calculate_fix_rate(&results);
        assert!((fix_rate - 66.666).abs() < 0.1);
    }
    
    #[test]
    fn test_generate_closure_report() {
        let agent = RetestAgent::new();
        
        let results = vec![
            RetestResult {
                finding_id: "1".to_string(),
                status: RetestStatus::Fixed,
                verified_at: chrono::Utc::now(),
                response_diff: None,
                notes: "".to_string(),
            },
            RetestResult {
                finding_id: "2".to_string(),
                status: RetestStatus::StillVulnerable,
                verified_at: chrono::Utc::now(),
                response_diff: None,
                notes: "".to_string(),
            },
        ];
        
        let report = agent.generate_closure_report(&results);
        
        assert_eq!(report.total, 2);
        assert_eq!(report.fixed, 1);
        assert_eq!(report.still_vulnerable, 1);
        assert_eq!(report.fix_rate, 50.0);
    }
}
