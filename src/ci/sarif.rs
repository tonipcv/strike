use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifReport {
    pub version: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    #[serde(rename = "informationUri")]
    pub information_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: Option<u32>,
}

pub struct SarifGenerator;

impl SarifGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate(&self, findings: &[SarifFinding]) -> SarifReport {
        let results: Vec<SarifResult> = findings.iter().map(|f| {
            SarifResult {
                rule_id: f.vuln_class.clone(),
                level: self.map_severity(&f.severity),
                message: SarifMessage {
                    text: f.description.clone(),
                },
                locations: vec![
                    SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: f.endpoint.clone(),
                            },
                            region: None,
                        },
                    }
                ],
            }
        }).collect();
        
        SarifReport {
            version: "2.1.0".to_string(),
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            runs: vec![
                SarifRun {
                    tool: SarifTool {
                        driver: SarifDriver {
                            name: "Strike Security".to_string(),
                            version: "0.2.0".to_string(),
                            information_uri: "https://github.com/strike-security/strike".to_string(),
                        },
                    },
                    results,
                }
            ],
        }
    }
    
    pub fn to_json(&self, report: &SarifReport) -> Result<String> {
        let json = serde_json::to_string_pretty(report)?;
        Ok(json)
    }
    
    fn map_severity(&self, severity: &str) -> String {
        match severity.to_lowercase().as_str() {
            "critical" => "error".to_string(),
            "high" => "error".to_string(),
            "medium" => "warning".to_string(),
            "low" => "note".to_string(),
            _ => "note".to_string(),
        }
    }
}

impl Default for SarifGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SarifFinding {
    pub vuln_class: String,
    pub severity: String,
    pub description: String,
    pub endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sarif_generator_creation() {
        let generator = SarifGenerator::new();
        assert_eq!(generator, SarifGenerator);
    }
    
    #[test]
    fn test_generate_sarif_report() {
        let generator = SarifGenerator::new();
        
        let findings = vec![
            SarifFinding {
                vuln_class: "SQLi".to_string(),
                severity: "Critical".to_string(),
                description: "SQL Injection vulnerability".to_string(),
                endpoint: "/api/users".to_string(),
            }
        ];
        
        let report = generator.generate(&findings);
        
        assert_eq!(report.version, "2.1.0");
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].results.len(), 1);
        assert_eq!(report.runs[0].results[0].rule_id, "SQLi");
        assert_eq!(report.runs[0].results[0].level, "error");
    }
    
    #[test]
    fn test_map_severity() {
        let generator = SarifGenerator::new();
        
        assert_eq!(generator.map_severity("Critical"), "error");
        assert_eq!(generator.map_severity("High"), "error");
        assert_eq!(generator.map_severity("Medium"), "warning");
        assert_eq!(generator.map_severity("Low"), "note");
    }
    
    #[test]
    fn test_to_json() {
        let generator = SarifGenerator::new();
        
        let findings = vec![
            SarifFinding {
                vuln_class: "XSS".to_string(),
                severity: "High".to_string(),
                description: "Cross-Site Scripting vulnerability".to_string(),
                endpoint: "/api/search".to_string(),
            }
        ];
        
        let report = generator.generate(&findings);
        let json = generator.to_json(&report);
        
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("2.1.0"));
        assert!(json_str.contains("XSS"));
    }
}
