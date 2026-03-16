use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Reconnaissance,
    Scanning,
    Exploitation,
    PostExploitation,
    Reporting,
    Utility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgs {
    pub target: String,
    pub options: HashMap<String, serde_json::Value>,
    pub timeout: Option<Duration>,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Xml,
    Text,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_name: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub findings: Vec<Finding>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub raw_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub category: FindingCategory,
    pub confidence: Confidence,
    pub cvss_score: Option<f64>,
    pub cve_id: Option<String>,
    pub affected_url: Option<String>,
    pub remediation: Option<String>,
    pub references: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindingCategory {
    Vulnerability,
    Misconfiguration,
    Information,
    Exposure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;
    
    /// Tool description
    fn description(&self) -> &str;
    
    /// Tool category
    fn category(&self) -> ToolCategory;
    
    /// Check if tool is installed and available
    async fn check_installation(&self) -> Result<bool>;
    
    /// Get installation instructions
    fn installation_instructions(&self) -> &str;
    
    /// Execute the tool with given arguments
    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput>;
    
    /// Parse raw tool output into structured findings
    fn parse_output(&self, raw_output: &str) -> Result<ToolResult>;
    
    /// Get the command that would be executed (for dry-run)
    fn get_command(&self, args: &ToolArgs) -> Result<Command>;
    
    /// Validate arguments before execution
    fn validate_args(&self, args: &ToolArgs) -> Result<()>;
    
    /// Get tool version
    async fn get_version(&self) -> Result<String> {
        Ok("unknown".to_string())
    }
    
    /// Check if tool requires elevated privileges
    fn requires_root(&self) -> bool {
        false
    }
    
    /// Get estimated execution time
    fn estimated_duration(&self, _args: &ToolArgs) -> Option<Duration> {
        None
    }
}

pub fn parse_severity(s: Option<&str>) -> Severity {
    match s {
        Some("critical") | Some("CRITICAL") => Severity::Critical,
        Some("high") | Some("HIGH") => Severity::High,
        Some("medium") | Some("MEDIUM") => Severity::Medium,
        Some("low") | Some("LOW") => Severity::Low,
        _ => Severity::Info,
    }
}

pub fn parse_confidence(s: Option<&str>) -> Confidence {
    match s {
        Some("high") | Some("HIGH") => Confidence::High,
        Some("medium") | Some("MEDIUM") => Confidence::Medium,
        _ => Confidence::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_parse_severity() {
        assert_eq!(parse_severity(Some("critical")), Severity::Critical);
        assert_eq!(parse_severity(Some("HIGH")), Severity::High);
        assert_eq!(parse_severity(Some("medium")), Severity::Medium);
        assert_eq!(parse_severity(None), Severity::Info);
    }
}
