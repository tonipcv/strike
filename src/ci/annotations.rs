use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAnnotation {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub annotation_level: String,
    pub message: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabAnnotation {
    pub description: String,
    pub severity: String,
    pub location: GitLabLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabLocation {
    pub path: String,
    pub lines: GitLabLines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabLines {
    pub begin: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnotationGenerator;

impl AnnotationGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate_github(&self, findings: &[AnnotationFinding]) -> Vec<GitHubAnnotation> {
        findings.iter().map(|f| {
            GitHubAnnotation {
                path: f.endpoint.clone(),
                start_line: 1,
                end_line: 1,
                annotation_level: self.map_severity_github(&f.severity),
                message: f.description.clone(),
                title: format!("{}: {}", f.vuln_class, f.endpoint),
            }
        }).collect()
    }
    
    pub fn generate_gitlab(&self, findings: &[AnnotationFinding]) -> Vec<GitLabAnnotation> {
        findings.iter().map(|f| {
            GitLabAnnotation {
                description: f.description.clone(),
                severity: self.map_severity_gitlab(&f.severity),
                location: GitLabLocation {
                    path: f.endpoint.clone(),
                    lines: GitLabLines { begin: 1 },
                },
            }
        }).collect()
    }
    
    fn map_severity_github(&self, severity: &str) -> String {
        match severity.to_lowercase().as_str() {
            "critical" => "failure".to_string(),
            "high" => "failure".to_string(),
            "medium" => "warning".to_string(),
            "low" => "notice".to_string(),
            _ => "notice".to_string(),
        }
    }
    
    fn map_severity_gitlab(&self, severity: &str) -> String {
        match severity.to_lowercase().as_str() {
            "critical" => "blocker".to_string(),
            "high" => "critical".to_string(),
            "medium" => "major".to_string(),
            "low" => "minor".to_string(),
            _ => "info".to_string(),
        }
    }
}

impl Default for AnnotationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AnnotationFinding {
    pub vuln_class: String,
    pub severity: String,
    pub description: String,
    pub endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_generator_creation() {
        let generator = AnnotationGenerator::new();
        assert_eq!(generator, AnnotationGenerator);
    }
    
    #[test]
    fn test_generate_github_annotations() {
        let generator = AnnotationGenerator::new();
        
        let findings = vec![
            AnnotationFinding {
                vuln_class: "SQLi".to_string(),
                severity: "Critical".to_string(),
                description: "SQL Injection found".to_string(),
                endpoint: "/api/users".to_string(),
            }
        ];
        
        let annotations = generator.generate_github(&findings);
        
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].annotation_level, "failure");
        assert_eq!(annotations[0].path, "/api/users");
    }
    
    #[test]
    fn test_generate_gitlab_annotations() {
        let generator = AnnotationGenerator::new();
        
        let findings = vec![
            AnnotationFinding {
                vuln_class: "XSS".to_string(),
                severity: "High".to_string(),
                description: "XSS vulnerability found".to_string(),
                endpoint: "/api/search".to_string(),
            }
        ];
        
        let annotations = generator.generate_gitlab(&findings);
        
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].severity, "critical");
        assert_eq!(annotations[0].location.path, "/api/search");
    }
    
    #[test]
    fn test_map_severity_github() {
        let generator = AnnotationGenerator::new();
        
        assert_eq!(generator.map_severity_github("Critical"), "failure");
        assert_eq!(generator.map_severity_github("High"), "failure");
        assert_eq!(generator.map_severity_github("Medium"), "warning");
        assert_eq!(generator.map_severity_github("Low"), "notice");
    }
    
    #[test]
    fn test_map_severity_gitlab() {
        let generator = AnnotationGenerator::new();
        
        assert_eq!(generator.map_severity_gitlab("Critical"), "blocker");
        assert_eq!(generator.map_severity_gitlab("High"), "critical");
        assert_eq!(generator.map_severity_gitlab("Medium"), "major");
        assert_eq!(generator.map_severity_gitlab("Low"), "minor");
    }
}
