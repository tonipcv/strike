use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub proof_of_concept: ProofOfConcept,
    pub metadata: EvidenceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfConcept {
    pub request: HttpTrace,
    pub response: HttpTrace,
    pub diff_evidence: Option<DiffEvidence>,
    pub replay_command: String,
    pub browser_timeline: Option<Vec<BrowserAction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpTrace {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub status_code: Option<u16>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEvidence {
    pub before: String,
    pub after: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub action_type: BrowserActionType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: String,
    pub screenshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserActionType {
    Navigate,
    Click,
    Input,
    Submit,
    Wait,
    Screenshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMetadata {
    pub captured_at: chrono::DateTime<chrono::Utc>,
    pub agent: String,
    pub validation_attempts: u32,
    pub confidence_score: f32,
}

impl Evidence {
    pub fn new(request: HttpTrace, response: HttpTrace, agent: String) -> Self {
        let replay_command = format!(
            "strike validate --finding-id <uuid> --replay"
        );

        Self {
            proof_of_concept: ProofOfConcept {
                request,
                response,
                diff_evidence: None,
                replay_command,
                browser_timeline: None,
            },
            metadata: EvidenceMetadata {
                captured_at: chrono::Utc::now(),
                agent,
                validation_attempts: 1,
                confidence_score: 0.0,
            },
        }
    }

    pub fn add_diff(&mut self, before: String, after: String, description: String) {
        self.proof_of_concept.diff_evidence = Some(DiffEvidence {
            before,
            after,
            description,
        });
    }

    pub fn add_browser_timeline(&mut self, timeline: Vec<BrowserAction>) {
        self.proof_of_concept.browser_timeline = Some(timeline);
    }

    pub fn set_confidence(&mut self, score: f32) {
        self.metadata.confidence_score = score.clamp(0.0, 1.0);
    }
}

impl HttpTrace {
    pub fn sanitize(&mut self) {
        let sensitive_headers = ["authorization", "cookie", "x-api-key", "x-auth-token"];
        
        for header in sensitive_headers {
            if let Some(value) = self.headers.get_mut(header) {
                *value = "[REDACTED]".to_string();
            }
        }

        if let Some(body) = &mut self.body {
            *body = Self::sanitize_body(body);
        }
    }

    fn sanitize_body(body: &str) -> String {
        let patterns = [
            (r#""password"\s*:\s*"[^"]*""#, r#""password":"[REDACTED]""#),
            (r#""token"\s*:\s*"[^"]*""#, r#""token":"[REDACTED]""#),
            (r#""api_key"\s*:\s*"[^"]*""#, r#""api_key":"[REDACTED]""#),
            (r#""secret"\s*:\s*"[^"]*""#, r#""secret":"[REDACTED]""#),
        ];

        let mut sanitized = body.to_string();
        for (pattern, replacement) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                sanitized = re.replace_all(&sanitized, replacement).to_string();
            }
        }
        sanitized
    }
}
