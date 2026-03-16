use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct HttpxWrapper {
    binary_path: PathBuf,
}

impl HttpxWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("httpx"),
        }
    }

    fn parse_httpx_json(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                let url = json["url"].as_str().unwrap_or("");
                let status_code = json["status_code"].as_i64().unwrap_or(0);
                let title = json["title"].as_str().unwrap_or("");
                let tech = json["tech"].as_array();

                let severity = if status_code == 200 || status_code == 301 || status_code == 302 {
                    Severity::Info
                } else if status_code == 401 || status_code == 403 {
                    Severity::Low
                } else if status_code >= 500 {
                    Severity::Medium
                } else {
                    Severity::Info
                };

                let mut description = format!("HTTP {} - {}", status_code, url);
                if !title.is_empty() {
                    description.push_str(&format!(" - Title: {}", title));
                }

                findings.push(Finding {
                    title: format!("Live Host: {}", url),
                    severity,
                    description,
                    category: FindingCategory::Information,
                    confidence: Confidence::High,
                    cvss_score: None,
                    cve_id: None,
                    affected_url: Some(url.to_string()),
                    remediation: None,
                    references: vec![],
                    metadata: {
                        let mut map = std::collections::HashMap::new();
                        map.insert("status_code".to_string(), serde_json::json!(status_code));
                        map.insert("title".to_string(), serde_json::json!(title));
                        if let Some(tech_arr) = tech {
                            map.insert("technologies".to_string(), serde_json::json!(tech_arr));
                        }
                        map
                    },
                });
            }
        }

        Ok(findings)
    }
}

impl Default for HttpxWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpxWrapper {
    fn name(&self) -> &str {
        "httpx"
    }

    fn description(&self) -> &str {
        "Fast HTTP toolkit for probing live hosts"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Reconnaissance
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("-version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install httpx:\n\
         go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing httpx against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("httpx execution timed out")??
        } else {
            cmd.output().await?
        };

        let duration = start.elapsed();

        Ok(ToolOutput {
            tool_name: self.name().to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            timestamp: chrono::Utc::now(),
        })
    }

    fn parse_output(&self, raw_output: &str) -> Result<ToolResult> {
        let findings = self.parse_httpx_json(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Target
        cmd.arg("-u").arg(&args.target);

        // JSON output
        cmd.arg("-json");

        // Silent mode
        cmd.arg("-silent");

        // Status code
        if args.options.get("status_code").and_then(|v| v.as_bool()).unwrap_or(true) {
            cmd.arg("-status-code");
        }

        // Title
        if args.options.get("title").and_then(|v| v.as_bool()).unwrap_or(true) {
            cmd.arg("-title");
        }

        // Tech detection
        if args.options.get("tech_detect").and_then(|v| v.as_bool()).unwrap_or(true) {
            cmd.arg("-tech-detect");
        }

        // Content length
        if args.options.get("content_length").and_then(|v| v.as_bool()).unwrap_or(false) {
            cmd.arg("-content-length");
        }

        // Follow redirects
        if args.options.get("follow_redirects").and_then(|v| v.as_bool()).unwrap_or(false) {
            cmd.arg("-follow-redirects");
        }

        // Threads
        if let Some(threads) = args.options.get("threads") {
            cmd.arg("-threads").arg(threads.as_u64().unwrap_or(50).to_string());
        } else {
            cmd.arg("-threads").arg("50");
        }

        // Timeout
        cmd.arg("-timeout").arg("10");

        // Retries
        cmd.arg("-retries").arg("1");

        Ok(cmd)
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        if args.target.is_empty() {
            return Err(anyhow!("Target is required"));
        }

        Ok(())
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("-version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.trim().to_string())
    }

    fn estimated_duration(&self, _args: &ToolArgs) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_httpx_installation() {
        let httpx = HttpxWrapper::new();
        let installed = httpx.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_parse_httpx_json() {
        let httpx = HttpxWrapper::new();
        let output = r#"{"url":"https://example.com","status_code":200,"title":"Example Domain","tech":["Nginx"]}"#;

        let result = httpx.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].metadata.get("status_code").unwrap().as_i64().unwrap(), 200);
    }
}
