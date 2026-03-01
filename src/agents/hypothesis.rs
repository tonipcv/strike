use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::llm::{
    provider::{LlmPrompt, LlmResponse, TaskClass},
    prompt::{EndpointInfo, PromptTemplate},
    router::LlmRouter,
};
use crate::models::vuln_class::VulnClass;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub endpoint: String,
    pub method: String,
    pub parameter: Option<String>,
    pub vuln_class: String,
    pub confidence: f32,
    pub severity_potential: String,
    pub reasoning: String,
    pub suggested_payload: Option<String>,
    pub test_strategy: String,
    pub owasp_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisOutput {
    pub hypotheses: Vec<Hypothesis>,
}

#[derive(Debug, Clone)]
pub struct EndpointGraph {
    pub endpoints: Vec<EndpointInfo>,
}

impl EndpointGraph {
    pub fn new(endpoints: Vec<EndpointInfo>) -> Self {
        Self { endpoints }
    }
    
    pub fn chunk(&self, batch_size: usize) -> Vec<Vec<EndpointInfo>> {
        self.endpoints
            .chunks(batch_size)
            .map(|chunk: &[EndpointInfo]| chunk.to_vec())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub auth_type: String,
    pub session_token: Option<String>,
    pub cookies: Vec<String>,
}

pub struct HypothesisAgent {
    llm_router: Arc<LlmRouter>,
    prompt_template: PromptTemplate,
    max_hypotheses: usize,
    batch_size: usize,
    max_concurrent_batches: usize,
}

impl HypothesisAgent {
    pub fn new(llm_router: Arc<LlmRouter>, max_hypotheses: Option<usize>) -> Result<Self> {
        Ok(Self {
            llm_router,
            prompt_template: PromptTemplate::new()?,
            max_hypotheses: max_hypotheses.unwrap_or(50),
            batch_size: 10,
            max_concurrent_batches: 4,
        })
    }
    
    pub async fn generate_hypotheses(
        &self,
        surface_model: EndpointGraph,
        auth_context: Option<SessionContext>,
        focus_classes: Vec<VulnClass>,
    ) -> Result<Vec<Hypothesis>> {
        let batches = surface_model.chunk(self.batch_size);
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_batches));
        
        let mut tasks = Vec::new();
        
        for batch in batches {
            let sem = semaphore.clone();
            let llm = self.llm_router.clone();
            let template = self.prompt_template.clone();
            let auth_ctx = auth_context.clone();
            let focus = focus_classes.clone();
            
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::process_batch(llm, template, batch, auth_ctx, focus).await
            });
            
            tasks.push(task);
        }
        
        let mut all_hypotheses = Vec::new();
        
        for task in tasks {
            match task.await {
                Ok(Ok(hypotheses)) => all_hypotheses.extend(hypotheses),
                Ok(Err(e)) => {
                    tracing::warn!("Batch processing failed: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Task join failed: {}", e);
                }
            }
        }
        
        let deduplicated = self.deduplicate_hypotheses(all_hypotheses);
        let ranked = self.rank_hypotheses(deduplicated);
        
        Ok(ranked.into_iter().take(self.max_hypotheses).collect())
    }
    
    async fn process_batch(
        llm_router: Arc<LlmRouter>,
        template: PromptTemplate,
        batch: Vec<EndpointInfo>,
        auth_context: Option<SessionContext>,
        focus_classes: Vec<VulnClass>,
    ) -> Result<Vec<Hypothesis>> {
        let auth_str = auth_context
            .as_ref()
            .map(|ctx| format!("{} - Token: {}", ctx.auth_type, ctx.session_token.as_deref().unwrap_or("none")));
        
        let focus_class_names: Vec<String> = focus_classes
            .iter()
            .map(|vc| format!("{:?}", vc))
            .collect();
        
        let prompt_text = template.render_hypothesis_generation(
            &batch,
            auth_str.as_deref(),
            &focus_class_names,
        )?;
        
        let prompt = LlmPrompt::new(prompt_text)
            .with_temperature(0.3)
            .with_max_tokens(4096)
            .with_json_mode(true);
        
        let response: LlmResponse = llm_router
            .complete_with_task_class(prompt, TaskClass::VulnHypothesis)
            .await
            .context("Failed to generate hypotheses from LLM")?;
        
        let output: HypothesisOutput = serde_json::from_str(&response.content)
            .context("Failed to parse LLM response as HypothesisOutput")?;
        
        let hypotheses_with_ids: Vec<Hypothesis> = output.hypotheses
            .into_iter()
            .map(|mut h| {
                if h.id.is_empty() || h.id == "uuid-v4" {
                    h.id = Uuid::new_v4().to_string();
                }
                h
            })
            .collect();
        
        Ok(hypotheses_with_ids)
    }
    
    pub fn deduplicate_hypotheses(&self, hypotheses: Vec<Hypothesis>) -> Vec<Hypothesis> {
        let mut seen = std::collections::HashSet::new();
        let mut deduplicated = Vec::new();
        
        for hypothesis in hypotheses {
            let key = format!("{}:{}:{}", hypothesis.endpoint, hypothesis.method, hypothesis.vuln_class);
            
            if !seen.contains(&key) {
                seen.insert(key);
                deduplicated.push(hypothesis);
            }
        }
        
        deduplicated
    }
    
    pub fn rank_hypotheses(&self, mut hypotheses: Vec<Hypothesis>) -> Vec<Hypothesis> {
        hypotheses.sort_by(|a, b| {
            let score_a = self.calculate_priority_score(a);
            let score_b = self.calculate_priority_score(b);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        hypotheses
    }
    
    pub fn chunk_hypotheses(&self, hypotheses: Vec<Hypothesis>) -> Vec<Vec<Hypothesis>> {
        hypotheses
            .chunks(self.batch_size)
            .map(|c| c.to_vec())
            .collect()
    }
    
    fn calculate_priority_score(&self, hypothesis: &Hypothesis) -> f32 {
        let severity_weight = match hypothesis.severity_potential.as_str() {
            "critical" => 4.0,
            "high" => 3.0,
            "medium" => 2.0,
            "low" => 1.0,
            _ => 1.5,
        };
        
        hypothesis.confidence * severity_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::router::RouterConfig;

    #[test]
    fn test_endpoint_graph_chunking() {
        let endpoints = vec![
            EndpointInfo {
                url: "/api/users/1".to_string(),
                method: "GET".to_string(),
                parameters: vec!["id".to_string()],
                auth_required: true,
                response_codes: vec![200],
            },
            EndpointInfo {
                url: "/api/users/2".to_string(),
                method: "GET".to_string(),
                parameters: vec!["id".to_string()],
                auth_required: true,
                response_codes: vec![200],
            },
        ];
        
        let graph = EndpointGraph::new(endpoints);
        let chunks = graph.chunk(1);
        
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 1);
    }
    
    #[test]
    fn test_hypothesis_deduplication() {
        let config = RouterConfig::default();
        if let Ok(router) = LlmRouter::new(config) {
            let agent = HypothesisAgent::new(Arc::new(router), Some(50)).unwrap();
            
            let hypotheses = vec![
                Hypothesis {
                    id: "1".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.9,
                    severity_potential: "high".to_string(),
                    reasoning: "Test".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "2".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.8,
                    severity_potential: "high".to_string(),
                    reasoning: "Test 2".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
            ];
            
            let deduplicated = agent.deduplicate_hypotheses(hypotheses);
            assert_eq!(deduplicated.len(), 1);
        }
    }
    
    #[test]
    fn test_hypothesis_ranking() {
        let config = RouterConfig::default();
        if let Ok(router) = LlmRouter::new(config) {
            let agent = HypothesisAgent::new(Arc::new(router), Some(50)).unwrap();
            
            let hypotheses = vec![
                Hypothesis {
                    id: "1".to_string(),
                    endpoint: "/api/users".to_string(),
                    method: "GET".to_string(),
                    parameter: Some("id".to_string()),
                    vuln_class: "IDOR".to_string(),
                    confidence: 0.5,
                    severity_potential: "low".to_string(),
                    reasoning: "Test".to_string(),
                    suggested_payload: None,
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A01:2021".to_string(),
                },
                Hypothesis {
                    id: "2".to_string(),
                    endpoint: "/api/admin".to_string(),
                    method: "POST".to_string(),
                    parameter: None,
                    vuln_class: "SQLi".to_string(),
                    confidence: 0.9,
                    severity_potential: "critical".to_string(),
                    reasoning: "Test 2".to_string(),
                    suggested_payload: Some("' OR 1=1--".to_string()),
                    test_strategy: "Test".to_string(),
                    owasp_ref: "A03:2021".to_string(),
                },
            ];
            
            let ranked = agent.rank_hypotheses(hypotheses);
            assert_eq!(ranked[0].id, "2");
            assert_eq!(ranked[1].id, "1");
        }
    }
}
