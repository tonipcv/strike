use anyhow::Result;
use crate::models::{Evidence, HttpTrace};

pub struct EvidenceAgent;

impl EvidenceAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn capture_evidence(&self, request: HttpTrace, response: HttpTrace) -> Evidence {
        Evidence::new(request, response, "EvidenceAgent".to_string())
    }

    pub fn sanitize_evidence(&self, mut evidence: Evidence) -> Evidence {
        evidence.proof_of_concept.request.sanitize();
        evidence.proof_of_concept.response.sanitize();
        evidence
    }

    pub fn validate_evidence_completeness(&self, evidence: &Evidence) -> f32 {
        let mut score = 0.0;
        let mut total = 5.0;

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
}
