use anyhow::Result;
use crate::models::{Evidence, HttpTrace, Target};
use crate::tools::HttpClient;
use std::collections::HashMap;

pub struct EvidenceAgent {
    http_client: HttpClient,
}

impl EvidenceAgent {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http_client: HttpClient::new(50, 30)?,
        })
    }

    pub async fn capture_http_evidence(&self, target: &Target, payload: &str) -> Result<Evidence> {
        // Build request URL with payload
        let test_url = if let Some(param) = &target.parameter {
            format!("{}?{}={}", target.full_url(), param, urlencoding::encode(payload))
        } else {
            format!("{}?test={}", target.full_url(), urlencoding::encode(payload))
        };

        // Execute request and capture trace
        let start = std::time::Instant::now();
        let response = self.http_client.get(&test_url).await?;
        let _duration = start.elapsed();

        // Build request trace
        let request_trace = HttpTrace {
            url: test_url.clone(),
            method: target.method.as_str().to_string(),
            headers: HashMap::new(),
            body: None,
            status_code: None,
            timestamp: chrono::Utc::now(),
        };

        // Build response trace
        let status_code = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let response_body = response.text().await?;

        let response_trace = HttpTrace {
            url: test_url.clone(),
            method: "RESPONSE".to_string(),
            headers: response_headers,
            body: Some(response_body.clone()),
            status_code: Some(status_code),
            timestamp: chrono::Utc::now(),
        };

        // Generate replay command (curl)
        let replay_command = self.generate_curl_command(&request_trace);

        // Create evidence
        let mut evidence = Evidence::new(request_trace, response_trace, "EvidenceAgent".to_string());
        evidence.proof_of_concept.replay_command = replay_command;
        evidence.metadata.confidence_score = if response_body.contains(payload) { 0.9 } else { 0.5 };

        Ok(evidence)
    }

    fn generate_curl_command(&self, request: &HttpTrace) -> String {
        let mut cmd = format!("curl -X {} '{}'", request.method, request.url);

        for (key, value) in &request.headers {
            cmd.push_str(&format!(" -H '{}: {}'", key, value));
        }

        if let Some(body) = &request.body {
            cmd.push_str(&format!(" -d '{}'", body.replace('\'', "'\\''")));
        }

        cmd
    }

    pub fn capture_evidence(&self, request: HttpTrace, response: HttpTrace) -> Evidence {
        Evidence::new(request, response, "EvidenceAgent".to_string())
    }

    pub fn sanitize_evidence(&self, mut evidence: Evidence) -> Evidence {
        evidence.proof_of_concept.request.sanitize();
        evidence.proof_of_concept.response.sanitize();
        
        // Remove sensitive data from headers
        let sensitive_headers = vec!["authorization", "cookie", "x-api-key", "x-auth-token"];
        for header in sensitive_headers {
            evidence.proof_of_concept.request.headers.remove(header);
            evidence.proof_of_concept.response.headers.remove(header);
        }
        
        evidence
    }

    pub fn validate_evidence_completeness(&self, evidence: &Evidence) -> f32 {
        let mut score = 0.0;
        let total = 5.0;

        if !evidence.proof_of_concept.request.url.is_empty() {
            score += 1.0;
        }

        if evidence.proof_of_concept.response.status_code.is_some() {
            score += 1.0;
        }

        if evidence.proof_of_concept.response.body.is_some() {
            score += 1.0;
        }

        if !evidence.proof_of_concept.replay_command.is_empty() {
            score += 1.0;
        }

        if evidence.metadata.confidence_score > 0.0 {
            score += 1.0;
        }

        score / total
    }

    pub async fn capture_screenshot(&self, _url: &str) -> Result<Vec<u8>> {
        // Placeholder for screenshot capture
        // In production, this would use headless browser (e.g., chromiumoxide)
        Ok(Vec::new())
    }

    pub fn generate_proof_of_concept(&self, evidence: &Evidence) -> String {
        let mut poc = String::new();
        
        poc.push_str("# Proof of Concept\n\n");
        poc.push_str(&format!("## Request\n```\n{}\n```\n\n", evidence.proof_of_concept.replay_command));
        
        if let Some(body) = &evidence.proof_of_concept.response.body {
            poc.push_str(&format!("## Response\n```\n{}\n```\n\n", 
                if body.len() > 500 { &body[..500] } else { body }
            ));
        }
        
        poc.push_str(&format!("## Confidence: {:.0}%\n", evidence.metadata.confidence_score * 100.0));
        
        poc
    }
}
