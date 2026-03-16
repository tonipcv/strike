use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct NiktoWrapper {
    binary_path: PathBuf,
}

impl NiktoWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("nikto"),
        }
    }

    fn parse_nikto_output(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for line in output.lines() {
            if line.starts_with('+') && !line.starts_with("+ Target") {
                let description = line.trim_start_matches('+').trim();
                
                let severity = if description.contains("OSVDB") || description.contains("CVE") {
                    Severity::High
                } else if description.contains("vulnerable") || description.contains("outdated") {
                    Severity::Medium
                } else {
                    Severity::Info
                };

                let finding = Finding {
                    title: "Nikto Finding".to_string(),
                    severity,
                    description: description.to_string(),
                    category: FindingCategory::Vulnerability,
                    confidence: Confidence::Medium,
                    cvss_score: None,
                    cve_id: None,
                    affected_url: None,
                    remediation: None,
                    references: vec![],
                    metadata: std::collections::HashMap::new(),
                };

                findings.push(finding);
            }
        }

        Ok(findings)
    }
}

impl Default for NiktoWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NiktoWrapper {
    fn name(&self) -> &str {
        "nikto"
    }

    fn description(&self) -> &str {
        "Web server scanner for vulnerabilities and misconfigurations"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("-Version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install Nikto:\n\
         Ubuntu/Debian: sudo apt-get install nikto\n\
         macOS: brew install nikto\n\
         Manual: git clone https://github.com/sullo/nikto.git"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing nikto against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("Nikto execution timed out")??
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
        let findings = self.parse_nikto_output(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Host
        cmd.arg("-h").arg(&args.target);

        // Tuning options
        if let Some(tuning) = args.options.get("tuning") {
            cmd.arg("-Tuning").arg(tuning.as_str().unwrap_or("x"));
        }

        // SSL
        if args.target.starts_with("https://") {
            cmd.arg("-ssl");
        }

        // No interactive
        cmd.arg("-nointeractive");

        // Output format
        match args.output_format {
            OutputFormat::Xml => {
                cmd.arg("-Format").arg("xml");
            }
            OutputFormat::Html => {
                cmd.arg("-Format").arg("htm");
            }
            _ => {}
        }

        Ok(cmd)
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        if args.target.is_empty() {
            return Err(anyhow!("Target is required"));
        }

        if !args.target.starts_with("http://") && !args.target.starts_with("https://") {
            return Err(anyhow!("Target must be a valid URL (http:// or https://)"));
        }

        Ok(())
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("-Version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.lines().next().unwrap_or("unknown").to_string())
    }

    fn estimated_duration(&self, _args: &ToolArgs) -> Option<Duration> {
        Some(Duration::from_secs(300)) // Nikto typically takes 5-10 minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_nikto_installation() {
        let nikto = NiktoWrapper::new();
        let installed = nikto.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_nikto_validation() {
        let nikto = NiktoWrapper::new();
        
        let args = ToolArgs {
            target: "example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = nikto.validate_args(&args);
        assert!(result.is_err());

        let args = ToolArgs {
            target: "https://example.com".to_string(),
            options: std::collections::HashMap::new(),
            timeout: None,
            output_format: OutputFormat::Text,
        };

        let result = nikto.validate_args(&args);
        assert!(result.is_ok());
    }
}
