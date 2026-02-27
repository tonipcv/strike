use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::policy::{CiPolicy, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub findings: Vec<BaselineFinding>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFinding {
    pub id: String,
    pub vuln_class: String,
    pub endpoint: String,
    pub severity: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub exit_code: i32,
    pub total_findings: usize,
    pub new_findings: usize,
    pub blocked_findings: usize,
    pub ignored_findings: usize,
    pub reasons: Vec<String>,
}

pub struct PolicyGate {
    policy: CiPolicy,
    baseline: Option<Baseline>,
}

impl PolicyGate {
    pub fn new(policy: CiPolicy) -> Self {
        Self {
            policy,
            baseline: None,
        }
    }
    
    pub async fn load_baseline(&mut self, path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let baseline: Baseline = serde_json::from_str(&content)?;
        self.baseline = Some(baseline);
        Ok(())
    }
    
    pub async fn save_baseline(&self, findings: &[BaselineFinding], path: &Path) -> Result<()> {
        let baseline = Baseline {
            findings: findings.to_vec(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = serde_json::to_string_pretty(&baseline)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    
    pub fn evaluate(&self, findings: &[BaselineFinding]) -> GateResult {
        let mut passed = true;
        let mut reasons = Vec::new();
        let mut blocked_findings = 0;
        let mut ignored_findings = 0;
        let mut new_findings = 0;
        
        for finding in findings {
            if self.policy.is_class_ignored(&finding.vuln_class) {
                ignored_findings += 1;
                continue;
            }
            
            if self.is_in_baseline(finding) {
                continue;
            }
            
            new_findings += 1;
            
            let severity = self.parse_severity(&finding.severity);
            
            if self.policy.should_fail(&severity) {
                if self.policy.is_route_blocked(&finding.endpoint) {
                    blocked_findings += 1;
                    passed = false;
                    reasons.push(format!(
                        "{} finding on blocked route {}: {}",
                        finding.severity, finding.endpoint, finding.vuln_class
                    ));
                } else if !self.policy.block_routes.is_empty() {
                    continue;
                } else {
                    blocked_findings += 1;
                    passed = false;
                    reasons.push(format!(
                        "{} {} finding: {}",
                        finding.severity, finding.vuln_class, finding.endpoint
                    ));
                }
            }
        }
        
        let exit_code = if passed { 0 } else { 1 };
        
        GateResult {
            passed,
            exit_code,
            total_findings: findings.len(),
            new_findings,
            blocked_findings,
            ignored_findings,
            reasons,
        }
    }
    
    fn is_in_baseline(&self, finding: &BaselineFinding) -> bool {
        if let Some(baseline) = &self.baseline {
            baseline.findings.iter().any(|bf| {
                bf.fingerprint == finding.fingerprint ||
                (bf.vuln_class == finding.vuln_class && bf.endpoint == finding.endpoint)
            })
        } else {
            false
        }
    }
    
    fn parse_severity(&self, severity: &str) -> Severity {
        match severity.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            _ => Severity::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_gate_creation() {
        let policy = CiPolicy::default();
        let gate = PolicyGate::new(policy);
        assert!(gate.baseline.is_none());
    }
    
    #[test]
    fn test_evaluate_no_findings() {
        let policy = CiPolicy::default();
        let gate = PolicyGate::new(policy);
        
        let findings = vec![];
        let result = gate.evaluate(&findings);
        
        assert!(result.passed);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.total_findings, 0);
    }
    
    #[test]
    fn test_evaluate_critical_finding_fails() {
        let policy = CiPolicy::default();
        let gate = PolicyGate::new(policy);
        
        let findings = vec![
            BaselineFinding {
                id: "1".to_string(),
                vuln_class: "SQLi".to_string(),
                endpoint: "/api/users".to_string(),
                severity: "Critical".to_string(),
                fingerprint: "abc123".to_string(),
            }
        ];
        
        let result = gate.evaluate(&findings);
        
        assert!(!result.passed);
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.blocked_findings, 1);
    }
    
    #[test]
    fn test_evaluate_ignored_class() {
        let mut policy = CiPolicy::default();
        policy.ignore_classes = vec!["InfoDisclosure".to_string()];
        let gate = PolicyGate::new(policy);
        
        let findings = vec![
            BaselineFinding {
                id: "1".to_string(),
                vuln_class: "InfoDisclosure".to_string(),
                endpoint: "/api/info".to_string(),
                severity: "Low".to_string(),
                fingerprint: "xyz789".to_string(),
            }
        ];
        
        let result = gate.evaluate(&findings);
        
        assert!(result.passed);
        assert_eq!(result.ignored_findings, 1);
    }
    
    #[test]
    fn test_evaluate_blocked_route() {
        let mut policy = CiPolicy::default();
        policy.block_routes = vec!["/api/admin".to_string()];
        let gate = PolicyGate::new(policy);
        
        let findings = vec![
            BaselineFinding {
                id: "1".to_string(),
                vuln_class: "IDOR".to_string(),
                endpoint: "/api/admin/users".to_string(),
                severity: "High".to_string(),
                fingerprint: "def456".to_string(),
            }
        ];
        
        let result = gate.evaluate(&findings);
        
        assert!(!result.passed);
        assert_eq!(result.blocked_findings, 1);
    }
}
