use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlReport {
    pub title: String,
    pub summary: String,
    pub findings: Vec<HtmlFinding>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlFinding {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub project: String,
    pub summary: String,
    pub description: String,
    pub issue_type: String,
    pub priority: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

pub struct HtmlExporter;

impl HtmlExporter {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate(&self, report: &HtmlReport) -> Result<String> {
        let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", report.title));
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("h1 { color: #333; }\n");
        html.push_str(".finding { border: 1px solid #ddd; padding: 15px; margin: 10px 0; }\n");
        html.push_str(".critical { border-left: 5px solid #d32f2f; }\n");
        html.push_str(".high { border-left: 5px solid #f57c00; }\n");
        html.push_str(".medium { border-left: 5px solid #fbc02d; }\n");
        html.push_str(".low { border-left: 5px solid #388e3c; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        html.push_str(&format!("<h1>{}</h1>\n", report.title));
        html.push_str(&format!("<p>{}</p>\n", report.summary));
        html.push_str(&format!("<p>Generated: {}</p>\n", report.generated_at));
        
        html.push_str("<h2>Findings</h2>\n");
        
        for finding in &report.findings {
            let severity_class = finding.severity.to_lowercase();
            html.push_str(&format!("<div class=\"finding {}\">\n", severity_class));
            html.push_str(&format!("<h3>{}</h3>\n", finding.title));
            html.push_str(&format!("<p><strong>Severity:</strong> {}</p>\n", finding.severity));
            html.push_str(&format!("<p><strong>Description:</strong> {}</p>\n", finding.description));
            html.push_str(&format!("<p><strong>Remediation:</strong> {}</p>\n", finding.remediation));
            html.push_str("</div>\n");
        }
        
        html.push_str("</body>\n</html>");
        
        Ok(html)
    }
}

impl Default for HtmlExporter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JiraExporter {
    base_url: String,
}

impl JiraExporter {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
    
    pub fn create_issue(&self, issue: &JiraIssue) -> Result<String> {
        let payload = serde_json::json!({
            "fields": {
                "project": {
                    "key": issue.project
                },
                "summary": issue.summary,
                "description": issue.description,
                "issuetype": {
                    "name": issue.issue_type
                },
                "priority": {
                    "name": issue.priority
                },
                "labels": issue.labels
            }
        });
        
        Ok(payload.to_string())
    }
    
    pub fn map_severity_to_priority(&self, severity: &str) -> String {
        match severity.to_lowercase().as_str() {
            "critical" => "Highest".to_string(),
            "high" => "High".to_string(),
            "medium" => "Medium".to_string(),
            "low" => "Low".to_string(),
            _ => "Medium".to_string(),
        }
    }
}

pub struct GitHubExporter {
    repo: String,
}

impl GitHubExporter {
    pub fn new(repo: String) -> Self {
        Self { repo }
    }
    
    pub fn create_issue(&self, issue: &GitHubIssue) -> Result<String> {
        let payload = serde_json::json!({
            "title": issue.title,
            "body": issue.body,
            "labels": issue.labels,
            "assignees": issue.assignees
        });
        
        Ok(payload.to_string())
    }
    
    pub fn format_finding_as_issue(&self, finding: &HtmlFinding) -> GitHubIssue {
        let body = format!(
            "## Description\n{}\n\n## Severity\n{}\n\n## Remediation\n{}",
            finding.description,
            finding.severity,
            finding.remediation
        );
        
        let labels = vec![
            "security".to_string(),
            format!("severity:{}", finding.severity.to_lowercase()),
        ];
        
        GitHubIssue {
            title: finding.title.clone(),
            body,
            labels,
            assignees: vec![],
        }
    }
}

pub struct PdfExporter;

impl PdfExporter {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate_placeholder(&self, report: &HtmlReport) -> Result<Vec<u8>> {
        let content = format!(
            "PDF Report: {}\n\nSummary: {}\n\nFindings: {}\n\nGenerated: {}",
            report.title,
            report.summary,
            report.findings.len(),
            report.generated_at
        );
        
        Ok(content.into_bytes())
    }
}

impl Default for PdfExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_report() -> HtmlReport {
        HtmlReport {
            title: "Security Test Report".to_string(),
            summary: "Test summary".to_string(),
            findings: vec![
                HtmlFinding {
                    id: "1".to_string(),
                    title: "SQL Injection".to_string(),
                    severity: "Critical".to_string(),
                    description: "SQL injection vulnerability found".to_string(),
                    remediation: "Use parameterized queries".to_string(),
                }
            ],
            generated_at: "2026-02-27".to_string(),
        }
    }

    #[test]
    fn test_html_exporter() {
        let exporter = HtmlExporter::new();
        let report = create_test_report();
        
        let html = exporter.generate(&report);
        assert!(html.is_ok());
        
        let html = html.unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Security Test Report"));
        assert!(html.contains("SQL Injection"));
    }
    
    #[test]
    fn test_jira_exporter() {
        let exporter = JiraExporter::new("https://jira.example.com".to_string());
        
        let issue = JiraIssue {
            project: "SEC".to_string(),
            summary: "SQL Injection vulnerability".to_string(),
            description: "Found SQL injection".to_string(),
            issue_type: "Bug".to_string(),
            priority: "High".to_string(),
            labels: vec!["security".to_string()],
        };
        
        let payload = exporter.create_issue(&issue);
        assert!(payload.is_ok());
        
        let payload = payload.unwrap();
        assert!(payload.contains("SEC"));
        assert!(payload.contains("SQL Injection"));
    }
    
    #[test]
    fn test_jira_severity_mapping() {
        let exporter = JiraExporter::new("https://jira.example.com".to_string());
        
        assert_eq!(exporter.map_severity_to_priority("Critical"), "Highest");
        assert_eq!(exporter.map_severity_to_priority("High"), "High");
        assert_eq!(exporter.map_severity_to_priority("Medium"), "Medium");
        assert_eq!(exporter.map_severity_to_priority("Low"), "Low");
    }
    
    #[test]
    fn test_github_exporter() {
        let exporter = GitHubExporter::new("owner/repo".to_string());
        
        let issue = GitHubIssue {
            title: "Security vulnerability".to_string(),
            body: "Description".to_string(),
            labels: vec!["security".to_string()],
            assignees: vec![],
        };
        
        let payload = exporter.create_issue(&issue);
        assert!(payload.is_ok());
    }
    
    #[test]
    fn test_github_format_finding() {
        let exporter = GitHubExporter::new("owner/repo".to_string());
        
        let finding = HtmlFinding {
            id: "1".to_string(),
            title: "XSS Vulnerability".to_string(),
            severity: "High".to_string(),
            description: "XSS found".to_string(),
            remediation: "Sanitize input".to_string(),
        };
        
        let issue = exporter.format_finding_as_issue(&finding);
        
        assert_eq!(issue.title, "XSS Vulnerability");
        assert!(issue.body.contains("XSS found"));
        assert!(issue.labels.contains(&"security".to_string()));
    }
    
    #[test]
    fn test_pdf_exporter() {
        let exporter = PdfExporter::new();
        let report = create_test_report();
        
        let pdf = exporter.generate_placeholder(&report);
        assert!(pdf.is_ok());
        
        let pdf_bytes = pdf.unwrap();
        assert!(!pdf_bytes.is_empty());
    }
}
