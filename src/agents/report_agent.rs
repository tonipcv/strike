use anyhow::Result;
use crate::models::{Finding, RunState};
use serde_json;

pub struct ReportAgent;

impl ReportAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_json_report(&self, findings: &[Finding], run_state: &RunState) -> Result<String> {
        let report = serde_json::json!({
            "run_id": run_state.id,
            "target": run_state.target,
            "profile": run_state.profile.as_str(),
            "status": format!("{:?}", run_state.status),
            "findings_count": {
                "critical": run_state.findings_count.critical,
                "high": run_state.findings_count.high,
                "medium": run_state.findings_count.medium,
                "low": run_state.findings_count.low,
                "info": run_state.findings_count.info,
            },
            "findings": findings,
            "metrics": run_state.metrics,
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }

    pub fn generate_markdown_report(&self, findings: &[Finding], run_state: &RunState) -> Result<String> {
        let mut md = String::new();

        md.push_str(&format!("# Strike Security Validation Report\n\n"));
        md.push_str(&format!("**Run ID:** {}\n", run_state.id));
        md.push_str(&format!("**Target:** {}\n", run_state.target));
        md.push_str(&format!("**Profile:** {}\n", run_state.profile.as_str()));
        md.push_str(&format!("**Status:** {:?}\n\n", run_state.status));

        md.push_str("## Executive Summary\n\n");
        md.push_str(&format!("- **Critical:** {}\n", run_state.findings_count.critical));
        md.push_str(&format!("- **High:** {}\n", run_state.findings_count.high));
        md.push_str(&format!("- **Medium:** {}\n", run_state.findings_count.medium));
        md.push_str(&format!("- **Low:** {}\n", run_state.findings_count.low));
        md.push_str(&format!("- **Info:** {}\n\n", run_state.findings_count.info));

        md.push_str("## Findings\n\n");

        for finding in findings {
            md.push_str(&format!("### {} - {}\n\n", finding.severity.as_str().to_uppercase(), finding.title));
            md.push_str(&format!("**ID:** {}\n", finding.id));
            md.push_str(&format!("**Vulnerability Class:** {}\n", finding.vuln_class));
            md.push_str(&format!("**CVSS v4.0 Score:** {} ({})\n", finding.cvss_v4_score.score, finding.cvss_v4_score.severity));
            md.push_str(&format!("**Status:** {}\n\n", finding.status.as_str()));

            md.push_str("**Target:**\n");
            md.push_str(&format!("- URL: {}\n", finding.target.url));
            md.push_str(&format!("- Endpoint: {}\n", finding.target.endpoint));
            md.push_str(&format!("- Method: {}\n\n", finding.target.method.as_str()));

            md.push_str("**Remediation:**\n");
            md.push_str(&format!("{}\n\n", finding.remediation.summary));

            if let Some(owasp) = finding.vuln_class.owasp_top10_mapping() {
                md.push_str(&format!("**OWASP Top 10:** {}\n", owasp));
            }

            if let Some(api) = finding.vuln_class.owasp_api_top10_mapping() {
                md.push_str(&format!("**OWASP API Top 10:** {}\n", api));
            }

            md.push_str("\n---\n\n");
        }

        Ok(md)
    }

    pub fn generate_sarif_report(&self, findings: &[Finding], run_state: &RunState) -> Result<String> {
        let mut rules = Vec::new();
        let mut results = Vec::new();

        for finding in findings {
            let rule_id = format!("STRIKE-{}", finding.vuln_class.cwe_id().unwrap_or(0));
            
            rules.push(serde_json::json!({
                "id": rule_id,
                "name": finding.vuln_class.to_string(),
                "shortDescription": {
                    "text": finding.title
                },
                "fullDescription": {
                    "text": finding.remediation.summary
                },
                "defaultConfiguration": {
                    "level": match finding.severity {
                        crate::models::Severity::Critical | crate::models::Severity::High => "error",
                        crate::models::Severity::Medium => "warning",
                        _ => "note"
                    }
                }
            }));

            results.push(serde_json::json!({
                "ruleId": rule_id,
                "level": match finding.severity {
                    crate::models::Severity::Critical | crate::models::Severity::High => "error",
                    crate::models::Severity::Medium => "warning",
                    _ => "note"
                },
                "message": {
                    "text": finding.title
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": finding.target.endpoint
                        }
                    }
                }]
            }));
        }

        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "Strike",
                        "version": "0.1.0",
                        "informationUri": "https://github.com/xaseai/strike",
                        "rules": rules
                    }
                },
                "results": results
            }]
        });

        Ok(serde_json::to_string_pretty(&sarif)?)
    }
}
