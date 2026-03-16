use crate::tools::r#trait::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, warn};

pub struct NmapWrapper {
    binary_path: PathBuf,
}

impl NmapWrapper {
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("nmap"),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self {
            binary_path: path,
        }
    }

    fn parse_nmap_output(&self, output: &str) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Parse nmap output for open ports and services
        for line in output.lines() {
            if line.contains("/tcp") || line.contains("/udp") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let port_proto = parts[0];
                    let state = parts[1];
                    let service = parts.get(2).unwrap_or(&"unknown");

                    if state == "open" {
                        let finding = Finding {
                            title: format!("Open Port: {}", port_proto),
                            severity: Severity::Info,
                            description: format!(
                                "Port {} is open and running {}",
                                port_proto, service
                            ),
                            category: FindingCategory::Information,
                            confidence: Confidence::High,
                            cvss_score: None,
                            cve_id: None,
                            affected_url: None,
                            remediation: Some(format!(
                                "Review if port {} needs to be exposed. Consider firewall rules.",
                                port_proto
                            )),
                            references: vec![],
                            metadata: {
                                let mut map = std::collections::HashMap::new();
                                map.insert("port".to_string(), serde_json::json!(port_proto));
                                map.insert("state".to_string(), serde_json::json!(state));
                                map.insert("service".to_string(), serde_json::json!(service));
                                map
                            },
                        };
                        findings.push(finding);
                    }
                }
            }

            // Check for vulnerable services
            if line.contains("version") && (line.contains("vulnerable") || line.contains("outdated")) {
                findings.push(Finding {
                    title: "Potentially Vulnerable Service Detected".to_string(),
                    severity: Severity::Medium,
                    description: line.to_string(),
                    category: FindingCategory::Vulnerability,
                    confidence: Confidence::Medium,
                    cvss_score: None,
                    cve_id: None,
                    affected_url: None,
                    remediation: Some("Update the service to the latest version".to_string()),
                    references: vec![],
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(findings)
    }
}

impl Default for NmapWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NmapWrapper {
    fn name(&self) -> &str {
        "nmap"
    }

    fn description(&self) -> &str {
        "Network mapper - port scanning and service detection"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Reconnaissance
    }

    async fn check_installation(&self) -> Result<bool> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await?;

        Ok(output.status.success())
    }

    fn installation_instructions(&self) -> &str {
        "Install Nmap:\n\
         Ubuntu/Debian: sudo apt-get install nmap\n\
         macOS: brew install nmap\n\
         Windows: https://nmap.org/download.html"
    }

    async fn execute(&self, args: &ToolArgs) -> Result<ToolOutput> {
        self.validate_args(args)?;

        debug!("Executing nmap against {}", args.target);
        let start = Instant::now();
        let mut cmd = self.get_command(args)?;

        let output = if let Some(timeout) = args.timeout {
            tokio::time::timeout(timeout, cmd.output())
                .await
                .context("Nmap execution timed out")??
        } else {
            cmd.output().await?
        };

        let duration = start.elapsed();

        if !output.status.success() {
            warn!(
                "Nmap exited with status: {} - stderr: {}",
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
        let findings = self.parse_nmap_output(raw_output)?;

        Ok(ToolResult {
            findings,
            metadata: std::collections::HashMap::new(),
            raw_output: raw_output.to_string(),
        })
    }

    fn get_command(&self, args: &ToolArgs) -> Result<Command> {
        let mut cmd = Command::new(&self.binary_path);

        // Scan type
        if let Some(scan_type) = args.options.get("scan_type") {
            match scan_type.as_str() {
                Some("syn") => cmd.arg("-sS"),
                Some("connect") => cmd.arg("-sT"),
                Some("udp") => cmd.arg("-sU"),
                Some("comprehensive") => cmd.arg("-sV").arg("-sC"),
                _ => cmd.arg("-sV"),
            };
        } else {
            cmd.arg("-sV"); // Default: version detection
        }

        // Port range
        if let Some(ports) = args.options.get("ports") {
            cmd.arg("-p").arg(ports.as_str().unwrap_or("1-1000"));
        } else {
            cmd.arg("-p").arg("1-1000"); // Default common ports
        }

        // Timing
        if let Some(timing) = args.options.get("timing") {
            cmd.arg(format!("-T{}", timing.as_u64().unwrap_or(4)));
        } else {
            cmd.arg("-T4"); // Default aggressive timing
        }

        // Scripts
        if let Some(scripts) = args.options.get("scripts") {
            if let Some(scripts_arr) = scripts.as_array() {
                let script_list: Vec<String> = scripts_arr
                    .iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect();
                cmd.arg("--script").arg(script_list.join(","));
            }
        }

        // Output format
        match args.output_format {
            OutputFormat::Xml => {
                cmd.arg("-oX").arg("-");
            }
            OutputFormat::Json => {
                // Nmap doesn't support JSON natively, use XML
                cmd.arg("-oX").arg("-");
            }
            _ => {
                // Normal output
            }
        }

        // Target
        cmd.arg(&args.target);

        Ok(cmd)
    }

    fn validate_args(&self, args: &ToolArgs) -> Result<()> {
        if args.target.is_empty() {
            return Err(anyhow!("Target is required"));
        }

        // Validate target format (basic check)
        if args.target.contains(';') || args.target.contains('&') {
            return Err(anyhow!("Invalid target format"));
        }

        Ok(())
    }

    async fn get_version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }

    fn requires_root(&self) -> bool {
        true // SYN scan requires root
    }

    fn estimated_duration(&self, args: &ToolArgs) -> Option<Duration> {
        // Estimate based on port range and timing
        let port_count = if let Some(ports) = args.options.get("ports") {
            if let Some(port_str) = ports.as_str() {
                if port_str.contains('-') {
                    let parts: Vec<&str> = port_str.split('-').collect();
                    if parts.len() == 2 {
                        let start: u16 = parts[0].parse().unwrap_or(1);
                        let end: u16 = parts[1].parse().unwrap_or(1000);
                        (end - start + 1) as u64
                    } else {
                        1000
                    }
                } else {
                    port_str.split(',').count() as u64
                }
            } else {
                1000
            }
        } else {
            1000
        };

        let timing = args
            .options
            .get("timing")
            .and_then(|t| t.as_u64())
            .unwrap_or(4);

        // Rough estimate: faster timing = less time per port
        let seconds_per_port = match timing {
            0 => 5.0,  // Paranoid
            1 => 2.0,  // Sneaky
            2 => 1.0,  // Polite
            3 => 0.5,  // Normal
            4 => 0.2,  // Aggressive
            5 => 0.1,  // Insane
            _ => 0.5,
        };

        Some(Duration::from_secs_f64(port_count as f64 * seconds_per_port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires nmap installation
    async fn test_nmap_installation() {
        let nmap = NmapWrapper::new();
        let installed = nmap.check_installation().await.unwrap();
        assert!(installed);
    }

    #[tokio::test]
    #[ignore]
    async fn test_nmap_version() {
        let nmap = NmapWrapper::new();
        let version = nmap.get_version().await.unwrap();
        assert!(version.contains("Nmap"));
    }

    #[test]
    fn test_nmap_command_generation() {
        let nmap = NmapWrapper::new();
        let args = ToolArgs {
            target: "scanme.nmap.org".to_string(),
            options: {
                let mut map = std::collections::HashMap::new();
                map.insert("ports".to_string(), serde_json::json!("80,443"));
                map.insert("timing".to_string(), serde_json::json!(4));
                map
            },
            timeout: Some(Duration::from_secs(60)),
            output_format: OutputFormat::Text,
        };

        let cmd = nmap.get_command(&args).unwrap();
        // Command should be valid
    }

    #[test]
    fn test_parse_nmap_output() {
        let nmap = NmapWrapper::new();
        let output = r#"
Starting Nmap 7.94
Nmap scan report for scanme.nmap.org
PORT     STATE SERVICE VERSION
22/tcp   open  ssh     OpenSSH 6.6.1p1
80/tcp   open  http    Apache httpd 2.4.7
443/tcp  open  https   nginx
        "#;

        let result = nmap.parse_output(output).unwrap();
        assert!(!result.findings.is_empty());
        assert!(result.findings.iter().any(|f| f.title.contains("22")));
        assert!(result.findings.iter().any(|f| f.title.contains("80")));
    }
}
