use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub title: String,
    pub scan_date: String,
    pub target: String,
    pub overall_risk: RiskLevel,
    pub key_findings: Vec<KeyFinding>,
    pub metrics: ScanMetrics,
    pub recommendations: Vec<String>,
    pub compliance_status: ComplianceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Minimal,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "Critical",
            RiskLevel::High => "High",
            RiskLevel::Medium => "Medium",
            RiskLevel::Low => "Low",
            RiskLevel::Minimal => "Minimal",
        }
    }
    
    pub fn color_code(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "#8B0000",
            RiskLevel::High => "#FF4500",
            RiskLevel::Medium => "#FFA500",
            RiskLevel::Low => "#FFD700",
            RiskLevel::Minimal => "#32CD32",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFinding {
    pub title: String,
    pub severity: RiskLevel,
    pub description: String,
    pub impact: String,
    pub affected_components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMetrics {
    pub total_endpoints: usize,
    pub endpoints_scanned: usize,
    pub vulnerabilities_found: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub scan_duration_seconds: u64,
    pub coverage_percentage: f64,
}

impl ScanMetrics {
    pub fn calculate_risk_score(&self) -> f64 {
        let critical_weight = 10.0;
        let high_weight = 5.0;
        let medium_weight = 2.0;
        let low_weight = 0.5;
        
        (self.critical_count as f64 * critical_weight)
            + (self.high_count as f64 * high_weight)
            + (self.medium_count as f64 * medium_weight)
            + (self.low_count as f64 * low_weight)
    }
    
    pub fn overall_risk_level(&self) -> RiskLevel {
        if self.critical_count > 0 {
            RiskLevel::Critical
        } else if self.high_count > 5 {
            RiskLevel::High
        } else if self.high_count > 0 || self.medium_count > 10 {
            RiskLevel::Medium
        } else if self.medium_count > 0 || self.low_count > 20 {
            RiskLevel::Low
        } else {
            RiskLevel::Minimal
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub owasp_top_10: ComplianceScore,
    pub pci_dss: ComplianceScore,
    pub gdpr: ComplianceScore,
    pub hipaa: ComplianceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScore {
    pub compliant: bool,
    pub score_percentage: f64,
    pub issues_found: usize,
    pub recommendations: Vec<String>,
}

pub struct ExecutiveReportGenerator;

impl ExecutiveReportGenerator {
    pub fn generate(summary: &ExecutiveSummary) -> Result<String> {
        let mut report = String::new();
        
        report.push_str(&format!("# {}\n\n", summary.title));
        report.push_str(&format!("**Scan Date:** {}\n", summary.scan_date));
        report.push_str(&format!("**Target:** {}\n", summary.target));
        report.push_str(&format!("**Overall Risk:** {}\n\n", summary.overall_risk.as_str()));
        
        report.push_str("## Executive Summary\n\n");
        report.push_str(&format!(
            "The security assessment identified **{}** vulnerabilities across **{}** endpoints.\n\n",
            summary.metrics.vulnerabilities_found,
            summary.metrics.endpoints_scanned
        ));
        
        report.push_str("### Risk Distribution\n\n");
        report.push_str(&format!("- **Critical:** {}\n", summary.metrics.critical_count));
        report.push_str(&format!("- **High:** {}\n", summary.metrics.high_count));
        report.push_str(&format!("- **Medium:** {}\n", summary.metrics.medium_count));
        report.push_str(&format!("- **Low:** {}\n\n", summary.metrics.low_count));
        
        report.push_str("## Key Findings\n\n");
        for (i, finding) in summary.key_findings.iter().enumerate() {
            report.push_str(&format!("### {}. {} ({})\n\n", i + 1, finding.title, finding.severity.as_str()));
            report.push_str(&format!("**Description:** {}\n\n", finding.description));
            report.push_str(&format!("**Impact:** {}\n\n", finding.impact));
            report.push_str(&format!("**Affected Components:** {}\n\n", finding.affected_components.join(", ")));
        }
        
        report.push_str("## Recommendations\n\n");
        for (i, rec) in summary.recommendations.iter().enumerate() {
            report.push_str(&format!("{}. {}\n", i + 1, rec));
        }
        
        report.push_str("\n## Compliance Status\n\n");
        report.push_str(&format!("- **OWASP Top 10:** {} ({:.1}%)\n", 
            if summary.compliance_status.owasp_top_10.compliant { "✓" } else { "✗" },
            summary.compliance_status.owasp_top_10.score_percentage
        ));
        report.push_str(&format!("- **PCI DSS:** {} ({:.1}%)\n",
            if summary.compliance_status.pci_dss.compliant { "✓" } else { "✗" },
            summary.compliance_status.pci_dss.score_percentage
        ));
        
        Ok(report)
    }
    
    pub fn generate_html(summary: &ExecutiveSummary) -> Result<String> {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str(&format!("<title>{}</title>\n", summary.title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 40px; }\n");
        html.push_str("h1 { color: #333; }\n");
        html.push_str(".risk-critical { color: #8B0000; font-weight: bold; }\n");
        html.push_str(".risk-high { color: #FF4500; font-weight: bold; }\n");
        html.push_str(".metric { background: #f5f5f5; padding: 10px; margin: 10px 0; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        html.push_str(&format!("<h1>{}</h1>\n", summary.title));
        html.push_str(&format!("<p><strong>Target:</strong> {}</p>\n", summary.target));
        html.push_str(&format!("<p><strong>Overall Risk:</strong> <span class=\"risk-{}\">{}</span></p>\n",
            summary.overall_risk.as_str().to_lowercase(),
            summary.overall_risk.as_str()
        ));
        
        html.push_str("<div class=\"metric\">\n");
        html.push_str(&format!("<h3>Vulnerabilities: {}</h3>\n", summary.metrics.vulnerabilities_found));
        html.push_str(&format!("<p>Critical: {}</p>\n", summary.metrics.critical_count));
        html.push_str(&format!("<p>High: {}</p>\n", summary.metrics.high_count));
        html.push_str("</div>\n");
        
        html.push_str("</body>\n</html>");
        
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_as_str() {
        assert_eq!(RiskLevel::Critical.as_str(), "Critical");
        assert_eq!(RiskLevel::High.as_str(), "High");
        assert_eq!(RiskLevel::Medium.as_str(), "Medium");
        assert_eq!(RiskLevel::Low.as_str(), "Low");
        assert_eq!(RiskLevel::Minimal.as_str(), "Minimal");
    }
    
    #[test]
    fn test_risk_level_color_code() {
        assert_eq!(RiskLevel::Critical.color_code(), "#8B0000");
        assert_eq!(RiskLevel::High.color_code(), "#FF4500");
    }
    
    #[test]
    fn test_scan_metrics_risk_score() {
        let metrics = ScanMetrics {
            total_endpoints: 100,
            endpoints_scanned: 100,
            vulnerabilities_found: 15,
            critical_count: 2,
            high_count: 3,
            medium_count: 5,
            low_count: 5,
            scan_duration_seconds: 300,
            coverage_percentage: 100.0,
        };
        
        let score = metrics.calculate_risk_score();
        assert!(score > 0.0);
    }
    
    #[test]
    fn test_scan_metrics_overall_risk_critical() {
        let metrics = ScanMetrics {
            total_endpoints: 100,
            endpoints_scanned: 100,
            vulnerabilities_found: 1,
            critical_count: 1,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            scan_duration_seconds: 300,
            coverage_percentage: 100.0,
        };
        
        assert_eq!(metrics.overall_risk_level(), RiskLevel::Critical);
    }
    
    #[test]
    fn test_scan_metrics_overall_risk_minimal() {
        let metrics = ScanMetrics {
            total_endpoints: 100,
            endpoints_scanned: 100,
            vulnerabilities_found: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            scan_duration_seconds: 300,
            coverage_percentage: 100.0,
        };
        
        assert_eq!(metrics.overall_risk_level(), RiskLevel::Minimal);
    }
}
