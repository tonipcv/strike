use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct SubfinderWrapper {
    binary_path: PathBuf,
}

impl SubfinderWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("subfinder"),
        }
    }

    fn parse_subfinder_json(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let mut subdomains = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some(host) = json["host"].as_str() {
                    subdomains.push(host.to_string());
                }
            } else {
                // Plain text output
                subdomains.push(line.trim().to_string());
            }
        }

        if !subdomains.is_empty() {
            findings.push(Finding {
                title: format!("Discovered {} Subdomains", subdomains.len()),
                severity: Severity::Info,
                description: format!(
                    "Found {} subdomains: {}",
                    subdomains.len(),
                    subdomains.join(", ")
                ),
                category: FindingCategory::Information,
                confidence: Confidence::High,
                cvss_score: None,
                cve_id: None,
                affected_url: None,
                remediation: None,
                references: vec![],
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("subdomains".to_string(), serde_json::json!(subdomains));
                    map.insert("count".to_string(), serde_json::json!(subdomains.len()));
                    map
                },
            });
        }

        Ok(findings)
    }
}

impl Default for SubfinderWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SubfinderWrapper {
    fn name(&self) -> &str {
        "subfinder"
    }

    fn description(&self) -> &str {
        "Fast passive subdomain enumeration tool"
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
        "Install Subfinder:\n\
         go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing subfinder for {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("Subfinder execution timed out")??
        } else {
            cmd.output().await?
        };

        let duration = start.elapsed();

        if !output.status.success() {
            warn!(
                "Subfinder exited with status: {} - stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

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
        let findings = self.parse_subfinder_json(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Domain
        cmd.arg("-d").arg(&args.target);

        // Silent mode
        cmd.arg("-silent");

        // JSON output
        if matches!(args.output_format, OutputFormat::Json) {
            cmd.arg("-json");
        }

        // All sources
        if args.options.get("all").and_then(|v| v.as_bool()).unwrap_or(false) {
            cmd.arg("-all");
        }

        // Recursive
        if args.options.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false) {
            cmd.arg("-recursive");
        }

        // Threads
        if let Some(threads) = args.options.get("threads") {
            cmd.arg("-t").arg(threads.as_u64().unwrap_or(10).to_string());
        }

        // Timeout per source
        cmd.arg("-timeout").arg("30");

        // Max enumeration time
        cmd.arg("-max-time").arg("10");

        Ok(cmd)
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        if args.target.is_empty() {
            return Err(anyhow!("Domain is required"));
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

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        let all_sources = args.options.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        
        if all_sources {
            Some(Duration::from_secs(600)) // 10 minutes for all sources
        } else {
            Some(Duration::from_secs(180)) // 3 minutes for default sources
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_subfinder_installation() {
        let subfinder = SubfinderWrapper::new();
        let installed = subfinder.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_parse_subfinder_output() {
        let subfinder = SubfinderWrapper::new();
        let output = "www.example.com\napi.example.com\nmail.example.com";

        let result = subfinder.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 1);
        
        let metadata = &result.findings[0].metadata;
        let subdomains = metadata.get("subdomains").unwrap().as_array().unwrap();
        assert_eq!(subdomains.len(), 3);
    }

    #[test]
    fn test_parse_subfinder_json() {
        let subfinder = SubfinderWrapper::new();
        let output = r#"{"host":"www.example.com","source":"crtsh"}
{"host":"api.example.com","source":"virustotal"}"#;

        let result = subfinder.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 1);
    }
}
