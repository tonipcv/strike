use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::models::VulnClass;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub vulnerable: bool,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub payload_used: String,
    pub response_indicators: Vec<String>,
}

#[async_trait]
pub trait VulnerabilityDetector: Send + Sync {
    fn name(&self) -> &str;
    fn vuln_class(&self) -> &str;
    async fn detect(&self, target: String, payload: String) -> Result<DetectionResult>;
    fn get_payloads(&self) -> Vec<String>;
}

pub struct SqlInjectionDetector {
    payloads: Vec<String>,
}

impl SqlInjectionDetector {
    pub fn new() -> Self {
        Self {
            payloads: vec![
                "' OR '1'='1".to_string(),
                "admin' --".to_string(),
                "' OR 1=1--".to_string(),
                "1' UNION SELECT NULL--".to_string(),
                "' AND SLEEP(5)--".to_string(),
            ],
        }
    }
}

#[async_trait]
impl VulnerabilityDetector for SqlInjectionDetector {
    fn name(&self) -> &str {
        "SQL Injection Detector"
    }
    
    fn vuln_class(&self) -> &str {
        "A03:2021 - Injection"
    }
    
    async fn detect(&self, target: String, payload: String) -> Result<DetectionResult> {
        let client = reqwest::Client::new();
        let response = client.post(&target)
            .body(payload.clone())
            .send()
            .await?;
        
        let status = response.status();
        let body = response.text().await?;
        
        let mut evidence = Vec::new();
        let mut response_indicators = Vec::new();
        
        if body.contains("SQL syntax") {
            evidence.push("SQL syntax error in response".to_string());
            response_indicators.push("SQL syntax".to_string());
        }
        
        if body.contains("mysql_") || body.contains("ORA-") {
            evidence.push("Database error message detected".to_string());
            response_indicators.push("Database error".to_string());
        }
        
        if status.is_server_error() {
            evidence.push("Server error status code".to_string());
        }
        
        let vulnerable = !evidence.is_empty();
        let confidence = if vulnerable { 0.9 } else { 0.0 };
        
        Ok(DetectionResult {
            vulnerable,
            confidence,
            evidence,
            payload_used: payload.to_string(),
            response_indicators,
        })
    }
    
    fn get_payloads(&self) -> Vec<String> {
        self.payloads.clone()
    }
}

pub struct XssDetector {
    payloads: Vec<String>,
}

impl XssDetector {
    pub fn new() -> Self {
        Self {
            payloads: vec![
                "<script>alert(1)</script>".to_string(),
                "<img src=x onerror=alert(1)>".to_string(),
                "javascript:alert(1)".to_string(),
                "<svg onload=alert(1)>".to_string(),
            ],
        }
    }
}

#[async_trait]
impl VulnerabilityDetector for XssDetector {
    fn name(&self) -> &str {
        "XSS Detector"
    }
    
    fn vuln_class(&self) -> &str {
        "A03:2021 - Injection"
    }
    
    async fn detect(&self, target: String, payload: String) -> Result<DetectionResult> {
        let client = reqwest::Client::new();
        let response = client.get(format!("{}?input={}", target, payload))
            .send()
            .await?;
        
        let body = response.text().await?;
        
        let mut evidence = Vec::new();
        let mut response_indicators = Vec::new();
        
        if body.contains(&payload) {
            evidence.push("Payload reflected in response".to_string());
            response_indicators.push("Reflected input".to_string());
        }
        
        if body.contains("<script>") && body.contains(&payload) {
            evidence.push("Script tag with payload detected".to_string());
            response_indicators.push("Script injection".to_string());
        }
        
        let vulnerable = !evidence.is_empty();
        let confidence = if vulnerable { 0.85 } else { 0.0 };
        
        Ok(DetectionResult {
            vulnerable,
            confidence,
            evidence,
            payload_used: payload.to_string(),
            response_indicators,
        })
    }
    
    fn get_payloads(&self) -> Vec<String> {
        self.payloads.clone()
    }
}

pub struct SsrfDetector {
    payloads: Vec<String>,
}

impl SsrfDetector {
    pub fn new() -> Self {
        Self {
            payloads: vec![
                "http://169.254.169.254/latest/meta-data/".to_string(),
                "http://localhost:8080/admin".to_string(),
                "http://127.0.0.1:22".to_string(),
            ],
        }
    }
}

#[async_trait]
impl VulnerabilityDetector for SsrfDetector {
    fn name(&self) -> &str {
        "SSRF Detector"
    }
    
    fn vuln_class(&self) -> &str {
        "A10:2021 - Server-Side Request Forgery"
    }
    
    async fn detect(&self, target: String, payload: String) -> Result<DetectionResult> {
        let client = reqwest::Client::new();
        let response = client.post(&target)
            .json(&serde_json::json!({"url": payload}))
            .send()
            .await?;
        
        let body = response.text().await?;
        
        let mut evidence = Vec::new();
        let mut response_indicators = Vec::new();
        
        if body.contains("169.254") || body.contains("metadata") {
            evidence.push("Cloud metadata access detected".to_string());
            response_indicators.push("Metadata leak".to_string());
        }
        
        if body.len() > 1000 {
            evidence.push("Large response suggesting successful SSRF".to_string());
        }
        
        let vulnerable = !evidence.is_empty();
        let confidence = if vulnerable { 0.8 } else { 0.0 };
        
        Ok(DetectionResult {
            vulnerable,
            confidence,
            evidence,
            payload_used: payload.to_string(),
            response_indicators,
        })
    }
    
    fn get_payloads(&self) -> Vec<String> {
        self.payloads.clone()
    }
}

pub struct DetectorEngine {
    detectors: Vec<Box<dyn VulnerabilityDetector>>,
}

impl DetectorEngine {
    pub fn new() -> Self {
        let mut detectors: Vec<Box<dyn VulnerabilityDetector>> = Vec::new();
        detectors.push(Box::new(SqlInjectionDetector::new()));
        detectors.push(Box::new(XssDetector::new()));
        detectors.push(Box::new(SsrfDetector::new()));
        
        Self { detectors }
    }
    
    pub async fn scan(&self, target: &str) -> Result<Vec<DetectionResult>> {
        let mut all_results = Vec::new();
        
        for detector in &self.detectors {
            for payload in detector.get_payloads() {
                match detector.detect(target.to_string(), payload).await {
                    Ok(result) => {
                        if result.vulnerable {
                            all_results.push(result);
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        
        Ok(all_results)
    }
    
    pub fn detector_count(&self) -> usize {
        self.detectors.len()
    }
}

pub struct VulnDetector;

impl VulnDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_vulnerability_class(&self, pattern: &str) -> Option<VulnClass> {
        if pattern.contains("SQL") || pattern.contains("syntax error") {
            Some(VulnClass::SqlInjection)
        } else if pattern.contains("<script>") || pattern.contains("onerror") {
            Some(VulnClass::XssReflected)
        } else if pattern.contains("SSRF") || pattern.contains("localhost") {
            Some(VulnClass::Ssrf)
        } else {
            None
        }
    }
}

pub struct VulnDetectors {
    detectors: Vec<Box<dyn VulnerabilityDetector>>,
}

impl VulnDetectors {
    pub fn new() -> Self {
        let detectors: Vec<Box<dyn VulnerabilityDetector>> = vec![
            Box::new(SqlInjectionDetector::new()),
            Box::new(XssDetector::new()),
            Box::new(SsrfDetector::new()),
        ];
        
        Self { detectors }
    }
    
    pub async fn detect_all(&self, target: String) -> Result<Vec<DetectionResult>> {
        let mut results = Vec::new();
        
        for detector in &self.detectors {
            for payload in detector.get_payloads() {
                match detector.detect(target.clone(), payload).await {
                    Ok(result) => {
                        if result.vulnerable {
                            results.push(result);
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        
        Ok(results)
    }
    
    pub fn get_detector_count(&self) -> usize {
        self.detectors.len()
    }
}
