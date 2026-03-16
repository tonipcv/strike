use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct NucleiWrapper {
    binary_path: PathBuf,
    templates_dir: Option<PathBuf>,
}

impl NucleiWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("nuclei"),
            templates_dir: dirs::home_dir().map(|h| h.join("nuclei-templates")),
        }
    }

    pub fn with_templates_dir(mut self, dir: PathBuf) -> Self {
        self.templates_dir = Some(dir);
        self
    }

    async fn update_templates(&self) -> Result<()> {
        debug!("Updating Nuclei templates");
        Command::new(&self.binary_path)
            .arg("-update-templates")
            .output()
            .await?;
        Ok(())
    }

    fn parse_nuclei_json(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                let info = &json["info"];
                let template_id = json["template-id"].as_str().unwrap_or("unknown");
                let matched_at = json["matched-at"].as_str().unwrap_or("");

                let finding = Finding {
                    title: info["name"].as_str().unwrap_or(template_id).to_string(),
                    severity: parse_severity(info["severity"].as_str()),
                    description: info["description"]
                        .as_str()
                        .unwrap_or("No description")
                        .to_string(),
                    category: FindingCategory::Vulnerability,
                    confidence: Confidence::High,
                    cvss_score: info["classification"]["cvss-score"].as_f64(),
                    cve_id: info["classification"]["cve-id"]
                        .as_str()
                        .map(String::from),
                    affected_url: Some(matched_at.to_string()),
                    remediation: info["remediation"].as_str().map(String::from),
                    references: info["reference"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    metadata: {
                        let mut map = std::collections::HashMap::new();
                        map.insert("template_id".to_string(), serde_json::json!(template_id));
                        map.insert("type".to_string(), json["type"].clone());
                        if let Some(tags) = info["tags"].as_str() {
                            map.insert("tags".to_string(), serde_json::json!(tags));
                        }
                        map
                    },
                };

                findings.push(finding);
            }
        }

        Ok(findings)
    }
}

impl Default for NucleiWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NucleiWrapper {
    fn name(&self) -> &str {
        "nuclei"
    }

    fn description(&self) -> &str {
        "Fast vulnerability scanner based on templates"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("-version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install Nuclei:\n\
         go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest\n\
         \n\
         Update templates:\n\
         nuclei -update-templates"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing nuclei against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("Nuclei execution timed out")??
        } else {
            cmd.output().await?
        };

        let duration = start.elapsed();

        if !output.status.success() {
            warn!(
                "Nuclei exited with status: {} - stderr: {}",
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
        let findings = self.parse_nuclei_json(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // JSON output
        cmd.arg("-json");

        // Silent mode (no banner)
        cmd.arg("-silent");

        // Templates
        if let Some(templates) = args.options.get("templates") {
            if let Some(templates_arr) = templates.as_array() {
                for template in templates_arr {
                    if let Some(t) = template.as_str() {
                        cmd.arg("-t").arg(t);
                    }
                }
            } else if let Some(t) = templates.as_str() {
                cmd.arg("-t").arg(t);
            }
        }

        // Severity filter
        if let Some(severity) = args.options.get("severity") {
            cmd.arg("-severity").arg(severity.as_str().unwrap_or("critical,high"));
        }

        // Rate limit
        if let Some(rate_limit) = args.options.get("rate_limit") {
            cmd.arg("-rate-limit")
                .arg(rate_limit.as_u64().unwrap_or(150).to_string());
        } else {
            cmd.arg("-rate-limit").arg("150");
        }

        // Concurrency
        if let Some(concurrency) = args.options.get("concurrency") {
            cmd.arg("-c")
                .arg(concurrency.as_u64().unwrap_or(25).to_string());
        }

        // Retries
        cmd.arg("-retries").arg("1");

        // Timeout
        cmd.arg("-timeout").arg("5");

        // Target
        cmd.arg("-target").arg(&args.target);

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

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        // Nuclei is generally fast, estimate based on template count
        let template_count = args
            .options
            .get("templates")
            .and_then(|t| t.as_array())
            .map(|arr| arr.len())
            .unwrap_or(100);

        // Rough estimate: 0.5 seconds per template
        Some(Duration::from_secs((template_count as u64) / 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_nuclei_installation() {
        let nuclei = NucleiWrapper::new();
        let installed = nuclei.check_installation().await.unwrap();
        assert!(installed);
    }

    #[test]
    fn test_parse_nuclei_json() {
        let nuclei = NucleiWrapper::new();
        let output = r#"{"template-id":"CVE-2021-44228","info":{"name":"Apache Log4j RCE","severity":"critical","description":"Apache Log4j2 JNDI features do not protect against attacker controlled LDAP","classification":{"cvss-score":10.0,"cve-id":"CVE-2021-44228"}},"type":"http","matched-at":"https://example.com"}"#;

        let result = nuclei.parse_output(output).unwrap();
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].severity, Severity::Critical);
        assert_eq!(result.findings[0].cve_id, Some("CVE-2021-44228".to_string()));
    }
}
