use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct SQLMapWrapper {
    binary_path: PathBuf,
}

impl SQLMapWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("sqlmap"),
        }
    }

    fn parse_sqlmap_output(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        if output.contains("vulnerable") || output.contains("injectable") {
            let severity = if output.contains("time-based") || output.contains("boolean-based") {
                Severity::High
            } else {
                Severity::Critical
            };

            findings.push(Finding {
                title: "SQL Injection Vulnerability Detected".to_string(),
                severity,
                description: "SQLMap detected SQL injection vulnerability".to_string(),
                category: FindingCategory::Vulnerability,
                confidence: Confidence::High,
                cvss_score: Some(9.8),
                cve_id: None,
                affected_url: None,
                remediation: Some(
                    "Use parameterized queries or prepared statements. \
                     Validate and sanitize all user input. \
                     Implement proper input validation."
                        .to_string(),
                ),
                references: vec![
                    "https://owasp.org/www-community/attacks/SQL_Injection".to_string(),
                ],
                metadata: std::collections::HashMap::new(),
            });
        }

        // Parse database information
        if output.contains("back-end DBMS:") {
            for line in output.lines() {
                if line.contains("back-end DBMS:") {
                    findings.push(Finding {
                        title: "Database Information Disclosure".to_string(),
                        severity: Severity::Info,
                        description: line.trim().to_string(),
                        category: FindingCategory::Information,
                        confidence: Confidence::High,
                        cvss_score: None,
                        cve_id: None,
                        affected_url: None,
                        remediation: None,
                        references: vec![],
                        metadata: std::collections::HashMap::new(),
                    });
                }
            }
        }

        Ok(findings)
    }
}

impl Default for SQLMapWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SQLMapWrapper {
    fn name(&self) -> &str {
        "sqlmap"
    }

    fn description(&self) -> &str {
        "Automatic SQL injection and database takeover tool"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Exploitation
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install SQLMap:\n\
         pip install sqlmap\n\
         \n\
         Or download from:\n\
         git clone --depth 1 https://github.com/sqlmapproject/sqlmap.git"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing sqlmap against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("SQLMap execution timed out")??
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
        let findings = self.parse_sqlmap_output(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Target URL
        cmd.arg("-u").arg(&args.target);

        // Batch mode (no user interaction)
        if args.options.get("batch").and_then(|v| v.as_bool()).unwrap_or(true) {
            cmd.arg("--batch");
        }

        // Level (1-5, thoroughness)
        if let Some(level) = args.options.get("level") {
            cmd.arg("--level").arg(level.as_u64().unwrap_or(1).to_string());
        } else {
            cmd.arg("--level").arg("1"); // Default safe level
        }

        // Risk (1-3, aggressiveness)
        if let Some(risk) = args.options.get("risk") {
            cmd.arg("--risk").arg(risk.as_u64().unwrap_or(1).to_string());
        } else {
            cmd.arg("--risk").arg("1"); // Default safe risk
        }

        // Threads
        if let Some(threads) = args.options.get("threads") {
            cmd.arg("--threads").arg(threads.as_u64().unwrap_or(1).to_string());
        }

        // Tamper scripts
        if let Some(tamper) = args.options.get("tamper") {
            if let Some(tamper_arr) = tamper.as_array() {
                let tamper_list: Vec<String> = tamper_arr
                    .iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect();
                cmd.arg("--tamper").arg(tamper_list.join(","));
            }
        }

        // Random agent
        cmd.arg("--random-agent");

        // Timeout
        cmd.arg("--timeout").arg("30");

        // Retries
        cmd.arg("--retries").arg("1");

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
            .arg("--version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.lines().next().unwrap_or("unknown").to_string())
    }

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        let level = args
            .options
            .get("level")
            .and_then(|l| l.as_u64())
            .unwrap_or(1);

        // Estimate based on level: higher level = more time
        let base_time = match level {
            1 => 300,   // 5 minutes
            2 => 900,   // 15 minutes
            3 => 1800,  // 30 minutes
            4 => 3600,  // 1 hour
            5 => 7200,  // 2 hours
            _ => 300,
        };

        Some(Duration::from_secs(base_time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_sqlmap_installation() {
        let sqlmap = SQLMapWrapper::new();
        let installed = sqlmap.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_sqlmap_validation() {
        let sqlmap = SQLMapWrapper::new();

        let args = ToolArgs {
            target: "example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = sqlmap.validate_args(&args);
        assert!(result.is_err());

        let args = ToolArgs {
            target: "https://example.com/page?id=1".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = sqlmap.validate_args(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_sqlmap_output() {
        let sqlmap = SQLMapWrapper::new();
        let output = r#"
[INFO] testing 'MySQL >= 5.0.12 AND time-based blind'
[INFO] GET parameter 'id' appears to be 'MySQL >= 5.0.12 AND time-based blind' injectable
back-end DBMS: MySQL >= 5.0.12
        "#;

        let result = sqlmap.parse_output(output).unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.severity == Severity::High || f.severity == Severity::Critical));
    }
}
