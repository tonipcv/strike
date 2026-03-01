use strike_security::reporting::executive::{
    ExecutiveSummary, RiskLevel, KeyFinding, ScanMetrics, ComplianceStatus, ComplianceScore,
    ExecutiveReportGenerator,
};

#[test]
fn test_risk_level_as_str() {
    assert_eq!(RiskLevel::Critical.as_str(), "Critical");
    assert_eq!(RiskLevel::High.as_str(), "High");
    assert_eq!(RiskLevel::Medium.as_str(), "Medium");
    assert_eq!(RiskLevel::Low.as_str(), "Low");
    assert_eq!(RiskLevel::Minimal.as_str(), "Minimal");
}

#[test]
fn test_risk_level_color_codes() {
    assert_eq!(RiskLevel::Critical.color_code(), "#8B0000");
    assert_eq!(RiskLevel::High.color_code(), "#FF4500");
    assert_eq!(RiskLevel::Medium.color_code(), "#FFA500");
    assert_eq!(RiskLevel::Low.color_code(), "#FFD700");
    assert_eq!(RiskLevel::Minimal.color_code(), "#32CD32");
}

#[test]
fn test_scan_metrics_risk_score_calculation() {
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
    
    // 2*10 + 3*5 + 5*2 + 5*0.5 = 20 + 15 + 10 + 2.5 = 47.5
    assert_eq!(score, 47.5);
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
fn test_scan_metrics_overall_risk_high() {
    let metrics = ScanMetrics {
        total_endpoints: 100,
        endpoints_scanned: 100,
        vulnerabilities_found: 6,
        critical_count: 0,
        high_count: 6,
        medium_count: 0,
        low_count: 0,
        scan_duration_seconds: 300,
        coverage_percentage: 100.0,
    };
    
    assert_eq!(metrics.overall_risk_level(), RiskLevel::High);
}

#[test]
fn test_scan_metrics_overall_risk_medium() {
    let metrics = ScanMetrics {
        total_endpoints: 100,
        endpoints_scanned: 100,
        vulnerabilities_found: 11,
        critical_count: 0,
        high_count: 0,
        medium_count: 11,
        low_count: 0,
        scan_duration_seconds: 300,
        coverage_percentage: 100.0,
    };
    
    assert_eq!(metrics.overall_risk_level(), RiskLevel::Medium);
}

#[test]
fn test_scan_metrics_overall_risk_low() {
    let metrics = ScanMetrics {
        total_endpoints: 100,
        endpoints_scanned: 100,
        vulnerabilities_found: 21,
        critical_count: 0,
        high_count: 0,
        medium_count: 0,
        low_count: 21,
        scan_duration_seconds: 300,
        coverage_percentage: 100.0,
    };
    
    assert_eq!(metrics.overall_risk_level(), RiskLevel::Low);
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

#[test]
fn test_executive_report_generation() {
    let summary = create_test_summary();
    let report = ExecutiveReportGenerator::generate(&summary).unwrap();
    
    assert!(report.contains("Security Assessment Report"));
    assert!(report.contains("Executive Summary"));
    assert!(report.contains("Key Findings"));
}

#[test]
fn test_executive_report_has_metrics() {
    let summary = create_test_summary();
    let report = ExecutiveReportGenerator::generate(&summary).unwrap();
    
    assert!(report.contains("Critical:"));
    assert!(report.contains("High:"));
    assert!(report.contains("Medium:"));
    assert!(report.contains("Low:"));
}

#[test]
fn test_executive_report_has_recommendations() {
    let summary = create_test_summary();
    let report = ExecutiveReportGenerator::generate(&summary).unwrap();
    
    assert!(report.contains("Recommendations"));
}

#[test]
fn test_executive_report_has_compliance() {
    let summary = create_test_summary();
    let report = ExecutiveReportGenerator::generate(&summary).unwrap();
    
    assert!(report.contains("Compliance Status"));
    assert!(report.contains("OWASP Top 10"));
}

#[test]
fn test_executive_html_generation() {
    let summary = create_test_summary();
    let html = ExecutiveReportGenerator::generate_html(&summary).unwrap();
    
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html>"));
    assert!(html.contains("</html>"));
}

#[test]
fn test_executive_html_has_styling() {
    let summary = create_test_summary();
    let html = ExecutiveReportGenerator::generate_html(&summary).unwrap();
    
    assert!(html.contains("<style>"));
    assert!(html.contains("risk-critical"));
    assert!(html.contains("risk-high"));
}

#[test]
fn test_executive_html_has_metrics() {
    let summary = create_test_summary();
    let html = ExecutiveReportGenerator::generate_html(&summary).unwrap();
    
    assert!(html.contains("Vulnerabilities:"));
    assert!(html.contains("Critical:"));
}

#[test]
fn test_compliance_score_structure() {
    let score = ComplianceScore {
        compliant: true,
        score_percentage: 95.0,
        issues_found: 2,
        recommendations: vec!["Fix issue 1".to_string()],
    };
    
    assert!(score.compliant);
    assert_eq!(score.score_percentage, 95.0);
    assert_eq!(score.issues_found, 2);
}

#[test]
fn test_key_finding_structure() {
    let finding = KeyFinding {
        title: "SQL Injection".to_string(),
        severity: RiskLevel::Critical,
        description: "SQL injection vulnerability found".to_string(),
        impact: "Data breach possible".to_string(),
        affected_components: vec!["/api/users".to_string()],
    };
    
    assert_eq!(finding.title, "SQL Injection");
    assert_eq!(finding.severity, RiskLevel::Critical);
}

#[test]
fn test_scan_metrics_zero_risk_score() {
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
    
    assert_eq!(metrics.calculate_risk_score(), 0.0);
}

#[test]
fn test_scan_metrics_high_risk_score() {
    let metrics = ScanMetrics {
        total_endpoints: 100,
        endpoints_scanned: 100,
        vulnerabilities_found: 100,
        critical_count: 10,
        high_count: 20,
        medium_count: 30,
        low_count: 40,
        scan_duration_seconds: 300,
        coverage_percentage: 100.0,
    };
    
    let score = metrics.calculate_risk_score();
    assert!(score > 100.0);
}

fn create_test_summary() -> ExecutiveSummary {
    ExecutiveSummary {
        title: "Security Assessment Report".to_string(),
        scan_date: "2026-02-29".to_string(),
        target: "https://example.com".to_string(),
        overall_risk: RiskLevel::High,
        key_findings: vec![
            KeyFinding {
                title: "SQL Injection".to_string(),
                severity: RiskLevel::Critical,
                description: "SQL injection found".to_string(),
                impact: "Data breach".to_string(),
                affected_components: vec!["/api/users".to_string()],
            },
        ],
        metrics: ScanMetrics {
            total_endpoints: 100,
            endpoints_scanned: 100,
            vulnerabilities_found: 15,
            critical_count: 2,
            high_count: 5,
            medium_count: 5,
            low_count: 3,
            scan_duration_seconds: 300,
            coverage_percentage: 100.0,
        },
        recommendations: vec![
            "Implement input validation".to_string(),
            "Use parameterized queries".to_string(),
        ],
        compliance_status: ComplianceStatus {
            owasp_top_10: ComplianceScore {
                compliant: false,
                score_percentage: 75.0,
                issues_found: 5,
                recommendations: vec!["Fix A1".to_string()],
            },
            pci_dss: ComplianceScore {
                compliant: true,
                score_percentage: 95.0,
                issues_found: 1,
                recommendations: vec![],
            },
            gdpr: ComplianceScore {
                compliant: true,
                score_percentage: 90.0,
                issues_found: 2,
                recommendations: vec![],
            },
            hipaa: ComplianceScore {
                compliant: true,
                score_percentage: 92.0,
                issues_found: 1,
                recommendations: vec![],
            },
        },
    }
}
