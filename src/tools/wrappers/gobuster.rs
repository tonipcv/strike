use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct GobusterWrapper {
    binary_path: PathBuf,
}

impl GobusterWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("gobuster"),
        }
    }

    fn parse_gobuster_output(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for line in output.lines() {
            if line.contains("(Status:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let path = parts[0];
                    let status = parts.iter()
                        .find(|p| p.starts_with("(Status:"))
                        .and_then(|s| s.trim_start_matches("(Status:").trim_end_matches(')').parse::<u16>().ok())
                        .unwrap_or(0);

                    let severity = match status {
                        200..=299 => Severity::Info,
                        300..=399 => Severity::Low,
                        401 | 403 => Severity::Medium,
                        _ => Severity::Info,
                    };

                    findings.push(Finding {
                        title: format!("Directory/File Found: {}", path),
                        severity,
                        description: format!("Found {} with status code {}", path, status),
                        category: FindingCategory::Information,
                        confidence: Confidence::High,
                        cvss_score: None,
                        cve_id: None,
                        affected_url: Some(path.to_string()),
                        remediation: if status == 401 || status == 403 {
                            Some("Review access controls for this resource".to_string())
                        } else {
                            None
                        },
                        references: vec![],
                        metadata: {
                            let mut map = std::collections::HashMap::new();
                            map.insert("path".to_string(), serde_json::json!(path));
                            map.insert("status_code".to_string(), serde_json::json!(status));
                            map
                        },
                    });
                }
            }
        }

        Ok(findings)
    }
}

impl Default for GobusterWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GobusterWrapper {
    fn name(&self) -> &str {
        "gobuster"
    }

    fn description(&self) -> &str {
        "Directory/file and DNS bruteforcing tool"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Reconnaissance
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install Gobuster:\n\
         go install github.com/OJ/gobuster/v3@latest"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing gobuster against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("Gobuster execution timed out")??
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
        let findings = self.parse_gobuster_output(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Mode (dir, dns, vhost, etc.)
        let mode = args.options.get("mode")
            .and_then(|m| m.as_str())
            .unwrap_or("dir");
        cmd.arg(mode);

        // URL
        cmd.arg("-u").arg(&args.target);

        // Wordlist
        let wordlist = args.options.get("wordlist")
            .and_then(|w| w.as_str())
            .unwrap_or("/usr/share/wordlists/dirb/common.txt");
        cmd.arg("-w").arg(wordlist);

        // Threads
        if let Some(threads) = args.options.get("threads") {
            cmd.arg("-t").arg(threads.as_u64().unwrap_or(50).to_string());
        } else {
            cmd.arg("-t").arg("50");
        }

        // Status codes to match
        if let Some(status_codes) = args.options.get("status_codes") {
            cmd.arg("-s").arg(status_codes.as_str().unwrap_or("200,204,301,302,307,401,403"));
        }

        // Extensions
        if let Some(extensions) = args.options.get("extensions") {
            cmd.arg("-x").arg(extensions.as_str().unwrap_or("php,html,js"));
        }

        // Quiet mode
        cmd.arg("-q");

        // No error output
        cmd.arg("--no-error");

        // Timeout
        cmd.arg("--timeout").arg("10s");

        Ok(cmd)
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        if args.target.is_empty() {
            return Err(anyhow!("Target URL is required"));
        }

        if !args.target.starts_with("http://") && !args.target.starts_with("https://") {
            return Err(anyhow!("Target must be a valid URL"));
        }

        Ok(())
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.lines().next().unwrap_or("unknown").to_string())
    }

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        let threads = args.options.get("threads")
            .and_then(|t| t.as_u64())
            .unwrap_or(50);

        // Estimate based on wordlist size and threads
        // Assuming average wordlist of 5000 words
        let estimated_seconds = 5000 / threads;
        Some(Duration::from_secs(estimated_seconds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_gobuster_installation() {
        let gobuster = GobusterWrapper::new();
        let installed = gobuster.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_gobuster_validation() {
        let gobuster = GobusterWrapper::new();

        let args = ToolArgs {
            target: "example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = gobuster.validate_args(&args);
        assert!(result.is_err());

        let args = ToolArgs {
            target: "https://example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = gobuster.validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_gobuster_output() {
        let gobuster = GobusterWrapper::new();
        let output = r#"
/admin                (Status: 403) [Size: 1234]
/login                (Status: 200) [Size: 5678]
/api                  (Status: 301) [Size: 0]
        "#;

        let result = gobuster.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 3);
        assert!(result.findings.iter().any(|f| f.title.contains("/admin")));
    }
}
