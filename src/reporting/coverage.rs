use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub owasp_top10: OwaspCoverage,
    pub wstg: WstgCoverage,
    pub overall_score: f32,
    pub tested_categories: usize,
    pub total_categories: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspCoverage {
    pub categories: HashMap<String, CategoryCoverage>,
    pub coverage_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WstgCoverage {
    pub categories: HashMap<String, CategoryCoverage>,
    pub coverage_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCoverage {
    pub name: String,
    pub tested: bool,
    pub findings_count: usize,
    pub test_cases: Vec<String>,
}

pub struct CoverageTracker {
    owasp_categories: HashMap<String, String>,
    wstg_categories: HashMap<String, String>,
}

impl CoverageTracker {
    pub fn new() -> Self {
        let mut owasp_categories = HashMap::new();
        owasp_categories.insert("A01:2021".to_string(), "Broken Access Control".to_string());
        owasp_categories.insert("A02:2021".to_string(), "Cryptographic Failures".to_string());
        owasp_categories.insert("A03:2021".to_string(), "Injection".to_string());
        owasp_categories.insert("A04:2021".to_string(), "Insecure Design".to_string());
        owasp_categories.insert("A05:2021".to_string(), "Security Misconfiguration".to_string());
        owasp_categories.insert("A06:2021".to_string(), "Vulnerable Components".to_string());
        owasp_categories.insert("A07:2021".to_string(), "Authentication Failures".to_string());
        owasp_categories.insert("A08:2021".to_string(), "Software and Data Integrity".to_string());
        owasp_categories.insert("A09:2021".to_string(), "Logging Failures".to_string());
        owasp_categories.insert("A10:2021".to_string(), "SSRF".to_string());
        
        let mut wstg_categories = HashMap::new();
        wstg_categories.insert("WSTG-INFO".to_string(), "Information Gathering".to_string());
        wstg_categories.insert("WSTG-CONF".to_string(), "Configuration Management".to_string());
        wstg_categories.insert("WSTG-IDNT".to_string(), "Identity Management".to_string());
        wstg_categories.insert("WSTG-ATHN".to_string(), "Authentication".to_string());
        wstg_categories.insert("WSTG-ATHZ".to_string(), "Authorization".to_string());
        wstg_categories.insert("WSTG-SESS".to_string(), "Session Management".to_string());
        wstg_categories.insert("WSTG-INPV".to_string(), "Input Validation".to_string());
        wstg_categories.insert("WSTG-ERRH".to_string(), "Error Handling".to_string());
        wstg_categories.insert("WSTG-CRYP".to_string(), "Cryptography".to_string());
        wstg_categories.insert("WSTG-BUSL".to_string(), "Business Logic".to_string());
        
        Self {
            owasp_categories,
            wstg_categories,
        }
    }
    
    pub fn generate_report(&self, findings: &[CoverageFinding]) -> CoverageReport {
        let owasp_coverage = self.calculate_owasp_coverage(findings);
        let wstg_coverage = self.calculate_wstg_coverage(findings);
        
        let tested_categories = owasp_coverage.categories.values()
            .filter(|c| c.tested)
            .count();
        
        let total_categories = self.owasp_categories.len();
        
        let overall_score = (owasp_coverage.coverage_percentage + wstg_coverage.coverage_percentage) / 2.0;
        
        CoverageReport {
            owasp_top10: owasp_coverage,
            wstg: wstg_coverage,
            overall_score,
            tested_categories,
            total_categories,
        }
    }
    
    fn calculate_owasp_coverage(&self, findings: &[CoverageFinding]) -> OwaspCoverage {
        let mut categories = HashMap::new();
        
        for (id, name) in &self.owasp_categories {
            let category_findings: Vec<_> = findings.iter()
                .filter(|f| f.owasp_category.as_ref() == Some(id))
                .collect();
            
            categories.insert(id.clone(), CategoryCoverage {
                name: name.clone(),
                tested: !category_findings.is_empty(),
                findings_count: category_findings.len(),
                test_cases: category_findings.iter()
                    .map(|f| f.test_case.clone())
                    .collect(),
            });
        }
        
        let tested_count = categories.values().filter(|c| c.tested).count();
        let coverage_percentage = (tested_count as f32 / self.owasp_categories.len() as f32) * 100.0;
        
        OwaspCoverage {
            categories,
            coverage_percentage,
        }
    }
    
    fn calculate_wstg_coverage(&self, findings: &[CoverageFinding]) -> WstgCoverage {
        let mut categories = HashMap::new();
        
        for (id, name) in &self.wstg_categories {
            let category_findings: Vec<_> = findings.iter()
                .filter(|f| f.wstg_category.as_ref().map(|c| c.starts_with(id)).unwrap_or(false))
                .collect();
            
            categories.insert(id.clone(), CategoryCoverage {
                name: name.clone(),
                tested: !category_findings.is_empty(),
                findings_count: category_findings.len(),
                test_cases: category_findings.iter()
                    .map(|f| f.test_case.clone())
                    .collect(),
            });
        }
        
        let tested_count = categories.values().filter(|c| c.tested).count();
        let coverage_percentage = (tested_count as f32 / self.wstg_categories.len() as f32) * 100.0;
        
        WstgCoverage {
            categories,
            coverage_percentage,
        }
    }
    
    pub fn map_vuln_to_owasp(&self, vuln_class: &str) -> Option<String> {
        match vuln_class {
            "IDOR" | "BOLA" => Some("A01:2021".to_string()),
            "SQLi" | "XSS" | "CommandInjection" => Some("A03:2021".to_string()),
            "AuthBypass" => Some("A07:2021".to_string()),
            "SSRF" => Some("A10:2021".to_string()),
            "Deserialization" => Some("A08:2021".to_string()),
            _ => None,
        }
    }
    
    pub fn map_vuln_to_wstg(&self, vuln_class: &str) -> Option<String> {
        match vuln_class {
            "IDOR" | "BOLA" => Some("WSTG-ATHZ-01".to_string()),
            "SQLi" => Some("WSTG-INPV-05".to_string()),
            "XSS" => Some("WSTG-INPV-01".to_string()),
            "AuthBypass" => Some("WSTG-ATHN-01".to_string()),
            "CSRF" => Some("WSTG-SESS-05".to_string()),
            "SSRF" => Some("WSTG-INPV-19".to_string()),
            _ => None,
        }
    }
}

impl Default for CoverageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CoverageFinding {
    pub test_case: String,
    pub owasp_category: Option<String>,
    pub wstg_category: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tracker_creation() {
        let tracker = CoverageTracker::new();
        assert_eq!(tracker.owasp_categories.len(), 10);
        assert_eq!(tracker.wstg_categories.len(), 10);
    }
    
    #[test]
    fn test_map_vuln_to_owasp() {
        let tracker = CoverageTracker::new();
        
        assert_eq!(tracker.map_vuln_to_owasp("IDOR"), Some("A01:2021".to_string()));
        assert_eq!(tracker.map_vuln_to_owasp("SQLi"), Some("A03:2021".to_string()));
        assert_eq!(tracker.map_vuln_to_owasp("SSRF"), Some("A10:2021".to_string()));
    }
    
    #[test]
    fn test_map_vuln_to_wstg() {
        let tracker = CoverageTracker::new();
        
        assert_eq!(tracker.map_vuln_to_wstg("IDOR"), Some("WSTG-ATHZ-01".to_string()));
        assert_eq!(tracker.map_vuln_to_wstg("SQLi"), Some("WSTG-INPV-05".to_string()));
        assert_eq!(tracker.map_vuln_to_wstg("XSS"), Some("WSTG-INPV-01".to_string()));
    }
    
    #[test]
    fn test_generate_report_empty() {
        let tracker = CoverageTracker::new();
        let findings = vec![];
        
        let report = tracker.generate_report(&findings);
        
        assert_eq!(report.tested_categories, 0);
        assert_eq!(report.total_categories, 10);
        assert_eq!(report.owasp_top10.coverage_percentage, 0.0);
    }
    
    #[test]
    fn test_generate_report_with_findings() {
        let tracker = CoverageTracker::new();
        let findings = vec![
            CoverageFinding {
                test_case: "Test IDOR".to_string(),
                owasp_category: Some("A01:2021".to_string()),
                wstg_category: Some("WSTG-ATHZ-01".to_string()),
            },
            CoverageFinding {
                test_case: "Test SQLi".to_string(),
                owasp_category: Some("A03:2021".to_string()),
                wstg_category: Some("WSTG-INPV-05".to_string()),
            },
        ];
        
        let report = tracker.generate_report(&findings);
        
        assert_eq!(report.tested_categories, 2);
        assert!(report.owasp_top10.coverage_percentage > 0.0);
        assert!(report.wstg.coverage_percentage > 0.0);
    }
}
