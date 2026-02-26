use anyhow::Result;
use crate::models::{VulnClass, Evidence, HttpTrace, Target};
use crate::tools::HttpClient;
use std::collections::HashMap;

pub struct ValidationAgent {
    http_client: HttpClient,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_vulnerable: bool,
    pub confidence: f32,
    pub evidence: Option<Evidence>,
}

impl ValidationAgent {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http_client: HttpClient::new(50, 30)?,
        })
    }

    pub async fn validate_vulnerability(
        &self,
        target: &Target,
        vuln_class: &VulnClass,
    ) -> Result<ValidationResult> {
        match vuln_class {
            VulnClass::Idor | VulnClass::Bola => self.validate_idor(target).await,
            VulnClass::SqlInjection => self.validate_sqli(target).await,
            VulnClass::XssReflected | VulnClass::XssStored => self.validate_xss(target).await,
            VulnClass::Ssrf => self.validate_ssrf(target).await,
            _ => Ok(ValidationResult {
                is_vulnerable: false,
                confidence: 0.0,
                evidence: None,
            }),
        }
    }

    async fn validate_idor(&self, target: &Target) -> Result<ValidationResult> {
        let url = target.full_url();
        
        let response = self.http_client.get(&url).await?;
        let status = response.status();

        let request_trace = HttpTrace {
            method: "GET".to_string(),
            url: url.clone(),
            headers: HashMap::new(),
            body: None,
            status_code: None,
            timestamp: chrono::Utc::now(),
        };

        let response_trace = HttpTrace {
            method: "GET".to_string(),
            url: url.clone(),
            headers: HashMap::new(),
            body: Some(response.text().await?),
            status_code: Some(status.as_u16()),
            timestamp: chrono::Utc::now(),
        };

        let is_vulnerable = status.is_success();
        let confidence = if is_vulnerable { 0.7 } else { 0.0 };

        let evidence = if is_vulnerable {
            Some(Evidence::new(request_trace, response_trace, "ValidationAgent".to_string()))
        } else {
            None
        };

        Ok(ValidationResult {
            is_vulnerable,
            confidence,
            evidence,
        })
    }

    async fn validate_sqli(&self, target: &Target) -> Result<ValidationResult> {
        let payloads = vec![
            "' OR '1'='1",
            "' OR 1=1--",
            "admin'--",
            "' UNION SELECT NULL--",
        ];

        for payload in payloads {
            let url = if target.full_url().contains('?') {
                format!("{}&test={}", target.full_url(), payload)
            } else {
                format!("{}?test={}", target.full_url(), payload)
            };

            match self.http_client.get(&url).await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await?;

                    if body.contains("SQL") || body.contains("syntax error") || body.contains("mysql") {
                        let request_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: None,
                            status_code: None,
                            timestamp: chrono::Utc::now(),
                        };

                        let response_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: Some(body),
                            status_code: Some(status.as_u16()),
                            timestamp: chrono::Utc::now(),
                        };

                        return Ok(ValidationResult {
                            is_vulnerable: true,
                            confidence: 0.85,
                            evidence: Some(Evidence::new(request_trace, response_trace, "ValidationAgent".to_string())),
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(ValidationResult {
            is_vulnerable: false,
            confidence: 0.0,
            evidence: None,
        })
    }

    async fn validate_xss(&self, target: &Target) -> Result<ValidationResult> {
        let payloads = vec![
            "<script>alert('XSS')</script>",
            "<img src=x onerror=alert('XSS')>",
            "<svg onload=alert('XSS')>",
        ];

        for payload in payloads {
            let url = if target.full_url().contains('?') {
                format!("{}&test={}", target.full_url(), urlencoding::encode(payload))
            } else {
                format!("{}?test={}", target.full_url(), urlencoding::encode(payload))
            };

            match self.http_client.get(&url).await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await?;

                    if body.contains(payload) {
                        let request_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: None,
                            status_code: None,
                            timestamp: chrono::Utc::now(),
                        };

                        let response_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: Some(body),
                            status_code: Some(status.as_u16()),
                            timestamp: chrono::Utc::now(),
                        };

                        return Ok(ValidationResult {
                            is_vulnerable: true,
                            confidence: 0.9,
                            evidence: Some(Evidence::new(request_trace, response_trace, "ValidationAgent".to_string())),
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(ValidationResult {
            is_vulnerable: false,
            confidence: 0.0,
            evidence: None,
        })
    }

    async fn validate_ssrf(&self, target: &Target) -> Result<ValidationResult> {
        let payloads = vec![
            "http://localhost",
            "http://127.0.0.1",
            "http://169.254.169.254",
        ];

        for payload in payloads {
            let url = if target.full_url().contains('?') {
                format!("{}&url={}", target.full_url(), urlencoding::encode(payload))
            } else {
                format!("{}?url={}", target.full_url(), urlencoding::encode(payload))
            };

            match self.http_client.get(&url).await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await?;

                    if status.is_success() && (body.len() > 0) {
                        let request_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: None,
                            status_code: None,
                            timestamp: chrono::Utc::now(),
                        };

                        let response_trace = HttpTrace {
                            method: "GET".to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: Some(body),
                            status_code: Some(status.as_u16()),
                            timestamp: chrono::Utc::now(),
                        };

                        return Ok(ValidationResult {
                            is_vulnerable: true,
                            confidence: 0.75,
                            evidence: Some(Evidence::new(request_trace, response_trace, "ValidationAgent".to_string())),
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(ValidationResult {
            is_vulnerable: false,
            confidence: 0.0,
            evidence: None,
        })
    }
}
