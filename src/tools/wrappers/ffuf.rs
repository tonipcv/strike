use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct FfufWrapper {
    binary_path: PathBuf,
}

impl FfufWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("ffuf"),
        }
    }

    fn parse_ffuf_json(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        if let Ok(json) = serde_json::from_str::<Value>(output) {
            if let Some(results) = json["results"].as_array() {
                for result in results {
                    let url = result["url"].as_str().unwrap_or("");
                    let status = result["status"].as_i64().unwrap_or(0);
                    let length = result["length"].as_i64().unwrap_or(0);
                    let words = result["words"].as_i64().unwrap_or(0);

                    let severity = match status {
                        200..=299 => Severity::Info,
                        300..=399 => Severity::Low,
                        401 | 403 => Severity::Medium,
                        _ => Severity::Info,
                    };

                    findings.push(Finding {
                        title: format!("Discovered: {}", url),
                        severity,
                        description: format!(
                            "Found {} (Status: {}, Length: {}, Words: {})",
                            url, status, length, words
                        ),
                        category: FindingCategory::Information,
                        confidence: Confidence::High,
                        cvss_score: None,
                        cve_id: None,
                        affected_url: Some(url.to_string()),
                        remediation: if status == 401 || status == 403 {
                            Some("Review access controls for this resource".to_string())
                        } else {
                            None
                        },
                        references: vec![],
                        metadata: {
                            let mut map = std::collections::HashMap::new();
                            map.insert("status".to_string(), serde_json::json!(status));
                            map.insert("length".to_string(), serde_json::json!(length));
                            map.insert("words".to_string(), serde_json::json!(words));
                            map
                        },
                    });
                }
            }
        }

        Ok(findings)
    }
}

impl Default for FfufWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FfufWrapper {
    fn name(&self) -> &str {
        "ffuf"
    }

    fn description(&self) -> &str {
        "Fast web fuzzer written in Go"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Reconnaissance
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("-V")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install ffuf:\n\
         go install github.com/ffuf/ffuf/v2@latest"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing ffuf against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("ffuf execution timed out")??
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
        let findings = self.parse_ffuf_json(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // URL with FUZZ keyword
        let url = if args.target.contains("FUZZ") {
            args.target.clone()
        } else {
            format!("{}/FUZZ", args.target.trim_end_matches('/'))
        };
        cmd.arg("-u").arg(url);

        // Wordlist
        let wordlist = args.options.get("wordlist")
            .and_then(|w| w.as_str())
            .unwrap_or("/usr/share/wordlists/dirb/common.txt");
        cmd.arg("-w").arg(wordlist);

        // Match status codes
        if let Some(match_status) = args.options.get("match_status") {
            if let Some(status_arr) = match_status.as_array() {
                let statuses: Vec<String> = status_arr
                    .iter()
                    .filter_map(|s| s.as_i64().map(|n| n.to_string()))
                    .collect();
                cmd.arg("-mc").arg(statuses.join(","));
            } else if let Some(status_str) = match_status.as_str() {
                cmd.arg("-mc").arg(status_str);
            }
        } else {
            cmd.arg("-mc").arg("200,204,301,302,307,401,403");
        }

        // Threads
        if let Some(threads) = args.options.get("threads") {
            cmd.arg("-t").arg(threads.as_u64().unwrap_or(40).to_string());
        } else {
            cmd.arg("-t").arg("40");
        }

        // Rate limit
        if let Some(rate) = args.options.get("rate") {
            cmd.arg("-rate").arg(rate.as_u64().unwrap_or(0).to_string());
        }

        // Timeout
        cmd.arg("-timeout").arg("10");

        // Silent mode
        cmd.arg("-s");

        // JSON output
        if matches!(args.output_format, OutputFormat::Json) {
            cmd.arg("-json");
        }

        // Auto-calibration
        cmd.arg("-ac");

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
            .arg("-V")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.trim().to_string())
    }

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        let threads = args.options.get("threads")
            .and_then(|t| t.as_u64())
            .unwrap_or(40);

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
    async fn test_ffuf_installation() {
        let ffuf = FfufWrapper::new();
        let installed = ffuf.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_ffuf_validation() {
        let ffuf = FfufWrapper::new();

        let args = ToolArgs {
            target: "example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = ffuf.validate_args(&args);
        assert!(result.is_err());

        let args = ToolArgs {
            target: "https://example.com/FUZZ".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = ffuf.validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_ffuf_json() {
        let ffuf = FfufWrapper::new();
        let output = r#"{"results":[{"url":"https://example.com/admin","status":403,"length":1234,"words":100},{"url":"https://example.com/login","status":200,"length":5678,"words":500}]}"#;

        let result = ffuf.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 2);
        assert!(result.findings.iter().any(|f| f.title.contains("/admin")));
    }
}
