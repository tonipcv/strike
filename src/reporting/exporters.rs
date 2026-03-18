use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    
    pub fn generate(&self, report: &HtmlReport) -> Result<Vec<u8>> {
        use printpdf::*;
        
        let (doc, page1, layer1) = PdfDocument::new(
            &report.title,
            Mm(210.0),
            Mm(297.0),
            "Layer 1"
        );
        
        let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
        
        let current_layer = doc.get_page(page1).get_layer(layer1);
        
        let mut y_position = Mm(270.0);
        let left_margin = Mm(20.0);
        let _right_margin = Mm(190.0);
        
        current_layer.use_text(&report.title, 24.0, left_margin, y_position, &font_bold);
        y_position -= Mm(10.0);
        
        current_layer.use_text(
            &format!("Generated: {}", report.generated_at),
            10.0,
            left_margin,
            y_position,
            &font
        );
        y_position -= Mm(15.0);
        
        current_layer.use_text("Summary", 16.0, left_margin, y_position, &font_bold);
        y_position -= Mm(7.0);
        
        let summary_lines = self.wrap_text(&report.summary, 80);
        for line in summary_lines {
            current_layer.use_text(&line, 11.0, left_margin, y_position, &font);
            y_position -= Mm(5.0);
        }
        
        y_position -= Mm(10.0);
        
        current_layer.use_text(
            &format!("Findings ({})", report.findings.len()),
            16.0,
            left_margin,
            y_position,
            &font_bold
        );
        y_position -= Mm(10.0);
        
        for (idx, finding) in report.findings.iter().enumerate() {
            if y_position < Mm(30.0) {
                let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                let _current_layer = doc.get_page(new_page).get_layer(new_layer);
                y_position = Mm(270.0);
            }
            
            current_layer.use_text(
                &format!("{}. {}", idx + 1, finding.title),
                12.0,
                left_margin,
                y_position,
                &font_bold
            );
            y_position -= Mm(6.0);
            
            let severity_color = match finding.severity.to_lowercase().as_str() {
                "critical" => Color::Rgb(Rgb::new(0.82, 0.18, 0.18, None)),
                "high" => Color::Rgb(Rgb::new(0.96, 0.49, 0.0, None)),
                "medium" => Color::Rgb(Rgb::new(0.98, 0.75, 0.18, None)),
                "low" => Color::Rgb(Rgb::new(0.22, 0.56, 0.24, None)),
                _ => Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)),
            };
            
            current_layer.set_fill_color(severity_color);
            current_layer.use_text(
                &format!("Severity: {}", finding.severity),
                10.0,
                left_margin,
                y_position,
                &font
            );
            current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            y_position -= Mm(6.0);
            
            current_layer.use_text("Description:", 10.0, left_margin, y_position, &font_bold);
            y_position -= Mm(5.0);
            
            let desc_lines = self.wrap_text(&finding.description, 80);
            for line in desc_lines {
                current_layer.use_text(&line, 9.0, left_margin + Mm(5.0), y_position, &font);
                y_position -= Mm(4.5);
            }
            
            y_position -= Mm(3.0);
            current_layer.use_text("Remediation:", 10.0, left_margin, y_position, &font_bold);
            y_position -= Mm(5.0);
            
            let rem_lines = self.wrap_text(&finding.remediation, 80);
            for line in rem_lines {
                current_layer.use_text(&line, 9.0, left_margin + Mm(5.0), y_position, &font);
                y_position -= Mm(4.5);
            }
            
            y_position -= Mm(8.0);
        }
        
        let pdf_bytes = doc.save_to_bytes()?;
        Ok(pdf_bytes)
    }
    
    fn wrap_text(&self, text: &str, max_chars: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > max_chars {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                }
            }
            
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        lines
    }
    
    pub fn generate_placeholder(&self, report: &HtmlReport) -> Result<Vec<u8>> {
        self.generate(report)
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
