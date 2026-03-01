use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRecord {
    pub id: String,
    pub request: RecordedRequest,
    pub response: RecordedResponse,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub request: RecordedRequest,
    pub response_status: u16,
    pub response_body: String,
    pub response_time_ms: u64,
    pub mutation_applied: Option<String>,
}

// Public request type used by tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl From<HttpRequest> for RecordedRequest {
    fn from(value: HttpRequest) -> Self {
        let mut headers = HashMap::new();
        for (k, v) in value.headers {
            headers.insert(k, v);
        }
        RecordedRequest {
            method: value.method,
            url: value.url,
            headers,
            body: value.body,
        }
    }
}

impl From<&HttpRequest> for RecordedRequest {
    fn from(value: &HttpRequest) -> Self {
        let mut headers = HashMap::new();
        for (k, v) in &value.headers {
            headers.insert(k.clone(), v.clone());
        }
        RecordedRequest {
            method: value.method.clone(),
            url: value.url.clone(),
            headers,
            body: value.body.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiff {
    pub status_changed: bool,
    pub body_length_delta: i64,
    pub headers_changed: Vec<String>,
    pub auth_bypass_detected: bool,
}

#[derive(Debug, Clone)]
pub enum MutationStrategy {
    ParameterFuzzing,
    HeaderInjection,
    IdorIncrement,
    IdorDecrement,
    IdorUuidSubstitution,
    SqliClassic,
    SqliTimeBased,
    XssReflected,
    SsrfInternal,
    AuthBypass,
    MassAssignment,
    MethodSwapping,
}

pub struct TrafficReplayer {
    records: Vec<TrafficRecord>,
}

impl TrafficReplayer {
    pub async fn new() -> Result<Self> {
        Ok(Self { records: Vec::new() })
    }
    
    pub fn record(&mut self, request: RecordedRequest, response: RecordedResponse) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let record = TrafficRecord {
            id: id.clone(),
            request,
            response,
            timestamp: chrono::Utc::now(),
        };
        
        self.records.push(record);
        id
    }
    
    pub async fn replay(&self, record_id: &str) -> Result<RecordedResponse> {
        let record = self.records
            .iter()
            .find(|r| r.id == record_id)
            .ok_or_else(|| anyhow::anyhow!("Record not found"))?;
        
        Ok(record.response.clone())
    }
    
    pub async fn replay_with_mutations(&self, request: &HttpRequest, strategy: MutationStrategy) -> Result<Vec<ReplayResult>> {
        let mut results = Vec::new();
        
        match strategy {
            MutationStrategy::ParameterFuzzing => {
                // Mutate each parameter with common payloads
                let payloads = vec![
                    "' OR '1'='1", "admin' --", "<script>alert(1)</script>",
                    "../../../etc/passwd", "{{7*7}}", "${7*7}",
                ];
                
                for payload in payloads {
                    let mut mutated = request.clone();
                    mutated.body = Some(payload.to_string());
                    let client = reqwest::Client::new();
                    let mut reqb = client.post(&mutated.url);
                    for (k, v) in &mutated.headers { reqb = reqb.header(k, v); }
                    let response = reqb
                        .body(payload)
                        .send()
                        .await?;
                    
                    results.push(ReplayResult {
                        request: RecordedRequest::from(mutated),
                        response_status: response.status().as_u16(),
                        response_body: response.text().await?,
                        response_time_ms: 100,
                        mutation_applied: Some(payload.to_string()),
                    });
                }
            },
            MutationStrategy::HeaderInjection => {
                // Inject malicious headers
                let headers = vec![
                    ("X-Forwarded-For", "127.0.0.1"),
                    ("X-Original-URL", "/admin"),
                    ("X-Rewrite-URL", "/admin"),
                ];
                
                for (header_name, header_value) in headers {
                    let client = reqwest::Client::new();
                    let mut reqb = client.get(&request.url);
                    for (k, v) in &request.headers { reqb = reqb.header(k, v); }
                    let response = reqb
                        .header(header_name, header_value)
                        .send()
                        .await?;
                    
                    results.push(ReplayResult {
                        request: RecordedRequest::from(request.clone()),
                        response_status: response.status().as_u16(),
                        response_body: response.text().await?,
                        response_time_ms: 100,
                        mutation_applied: Some(format!("{}: {}", header_name, header_value)),
                    });
                }
            },
            MutationStrategy::MethodSwapping => {
                // Try different HTTP methods
                let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"];
                
                for method in methods {
                    let client = reqwest::Client::new();
                    let mut reqb = match method {
                        "GET" => client.get(&request.url),
                        "POST" => client.post(&request.url),
                        "PUT" => client.put(&request.url),
                        "DELETE" => client.delete(&request.url),
                        "PATCH" => client.patch(&request.url),
                        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &request.url),
                        _ => client.get(&request.url),
                    };
                    for (k, v) in &request.headers { reqb = reqb.header(k, v); }
                    let response = reqb.send().await?;
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    let body = if body.is_empty() { format!("HTTP {}", status) } else { body };
                    
                    results.push(ReplayResult {
                        request: RecordedRequest::from(request.clone()),
                        response_status: status,
                        response_body: body,
                        response_time_ms: 100,
                        mutation_applied: Some(format!("Method: {}", method)),
                    });
                }
            },
            MutationStrategy::AuthBypass => {
                // Try authentication bypass techniques
                let bypass_headers = vec![
                    ("X-Original-URL", "/admin"),
                    ("X-Custom-IP-Authorization", "127.0.0.1"),
                    ("X-Forwarded-For", "localhost"),
                ];
                
                for (header_name, header_value) in bypass_headers {
                    let client = reqwest::Client::new();
                    let mut reqb = client.get(&request.url);
                    for (k, v) in &request.headers { reqb = reqb.header(k, v); }
                    let response = reqb
                        .header(header_name, header_value)
                        .send()
                        .await?;
                    
                    results.push(ReplayResult {
                        request: RecordedRequest::from(request.clone()),
                        response_status: response.status().as_u16(),
                        response_body: response.text().await?,
                        response_time_ms: 100,
                        mutation_applied: Some(format!("Auth bypass: {}", header_name)),
                    });
                }
            },
            _ => {}
        }
        
        Ok(results)
    }
    
    pub fn mutate_param(&self, param: &str, strategy: MutationStrategy) -> Vec<String> {
        match strategy {
            MutationStrategy::IdorIncrement => {
                if let Ok(num) = param.parse::<i64>() {
                    vec![
                        (num + 1).to_string(),
                        (num + 10).to_string(),
                        (num + 100).to_string(),
                        (num - 1).to_string(),
                        (num - 10).to_string(),
                    ]
                } else {
                    vec![]
                }
            }
            MutationStrategy::IdorUuidSubstitution => {
                vec![
                    uuid::Uuid::new_v4().to_string(),
                    "00000000-0000-0000-0000-000000000000".to_string(),
                    "11111111-1111-1111-1111-111111111111".to_string(),
                ]
            }
            MutationStrategy::SqliClassic => {
                vec![
                    "' OR '1'='1".to_string(),
                    "' OR 1=1--".to_string(),
                    "admin'--".to_string(),
                    "' UNION SELECT NULL--".to_string(),
                ]
            }
            MutationStrategy::SqliTimeBased => {
                vec![
                    "'; WAITFOR DELAY '00:00:05'--".to_string(),
                    "' AND SLEEP(5)--".to_string(),
                ]
            }
            MutationStrategy::XssReflected => {
                vec![
                    "<script>alert(1)</script>".to_string(),
                    "<img src=x onerror=alert(1)>".to_string(),
                    "javascript:alert(1)".to_string(),
                ]
            }
            MutationStrategy::SsrfInternal => {
                vec![
                    "http://localhost".to_string(),
                    "http://127.0.0.1".to_string(),
                    "http://169.254.169.254".to_string(),
                    "http://metadata.google.internal".to_string(),
                ]
            }
            MutationStrategy::AuthBypass => {
                vec!["".to_string()]
            }
            MutationStrategy::MassAssignment => {
                vec![]
            }
            _ => vec![],
        }
    }
    
    pub fn diff_responses(&self, baseline: &RecordedResponse, mutated: &RecordedResponse) -> ResponseDiff {
        let status_changed = baseline.status_code != mutated.status_code;
        let body_length_delta = mutated.body.len() as i64 - baseline.body.len() as i64;
        
        let mut headers_changed = Vec::new();
        for (key, baseline_value) in &baseline.headers {
            if let Some(mutated_value) = mutated.headers.get(key) {
                if baseline_value != mutated_value {
                    headers_changed.push(key.clone());
                }
            }
        }
        
        let auth_bypass_detected = baseline.status_code == 401 && mutated.status_code == 200;
        
        ResponseDiff {
            status_changed,
            body_length_delta,
            headers_changed,
            auth_bypass_detected,
        }
    }
    
    pub fn export_to_curl(&self, record_id: &str) -> Result<String> {
        let record = self.records
            .iter()
            .find(|r| r.id == record_id)
            .ok_or_else(|| anyhow::anyhow!("Record not found"))?;
        
        let mut curl = format!("curl -X {} '{}'", record.request.method, record.request.url);
        
        for (key, value) in &record.request.headers {
            curl.push_str(&format!(" -H '{}: {}'", key, value));
        }
        
        if let Some(body) = &record.request.body {
            curl.push_str(&format!(" -d '{}'", body));
        }
        
        Ok(curl)
    }
    
    pub fn export_to_python_requests(&self, record_id: &str) -> Result<String> {
        let record = self.records
            .iter()
            .find(|r| r.id == record_id)
            .ok_or_else(|| anyhow::anyhow!("Record not found"))?;
        
        let mut python = String::from("import requests\n\n");
        python.push_str(&format!("url = '{}'\n", record.request.url));
        python.push_str("headers = {\n");
        
        for (key, value) in &record.request.headers {
            python.push_str(&format!("    '{}': '{}',\n", key, value));
        }
        
        python.push_str("}\n");
        
        if let Some(body) = &record.request.body {
            python.push_str(&format!("data = '{}'\n", body));
            python.push_str(&format!("response = requests.{}(url, headers=headers, data=data)\n", 
                                    record.request.method.to_lowercase()));
        } else {
            python.push_str(&format!("response = requests.{}(url, headers=headers)\n", 
                                    record.request.method.to_lowercase()));
        }
        
        python.push_str("print(response.status_code)\n");
        python.push_str("print(response.text)\n");
        
        Ok(python)
    }
}

impl Default for TrafficReplayer {
    fn default() -> Self {
        Self { records: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_traffic_replayer_creation() {
        let replayer = TrafficReplayer::new().await.unwrap();
        assert_eq!(replayer.records.len(), 0);
    }
    
    #[tokio::test]
    async fn test_record_traffic() {
        let mut replayer = TrafficReplayer::new().await.unwrap();
        
        let request = RecordedRequest {
            method: "GET".to_string(),
            url: "https://example.com/api/users/123".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        
        let response = RecordedResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "{}".to_string(),
            duration_ms: 100,
        };
        
        let id = replayer.record(request, response);
        assert!(!id.is_empty());
        assert_eq!(replayer.records.len(), 1);
    }
    
    #[tokio::test]
    async fn test_mutate_param_idor() {
        let replayer = TrafficReplayer::new().await.unwrap();
        let mutations = replayer.mutate_param("123", MutationStrategy::IdorIncrement);
        
        assert_eq!(mutations.len(), 5);
        assert!(mutations.contains(&"124".to_string()));
        assert!(mutations.contains(&"122".to_string()));
    }
    
    #[tokio::test]
    async fn test_mutate_param_sqli() {
        let replayer = TrafficReplayer::new().await.unwrap();
        let mutations = replayer.mutate_param("test", MutationStrategy::SqliClassic);
        
        assert!(!mutations.is_empty());
        assert!(mutations.iter().any(|m| m.contains("OR")));
    }
    
    #[tokio::test]
    async fn test_diff_responses() {
        let replayer = TrafficReplayer::new().await.unwrap();
        
        let baseline = RecordedResponse {
            status_code: 401,
            headers: HashMap::new(),
            body: "Unauthorized".to_string(),
            duration_ms: 50,
        };
        
        let mutated = RecordedResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "Success".to_string(),
            duration_ms: 100,
        };
        
        let diff = replayer.diff_responses(&baseline, &mutated);
        assert!(diff.status_changed);
        assert!(diff.auth_bypass_detected);
    }
    
    #[tokio::test]
    async fn test_export_to_curl() {
        let mut replayer = TrafficReplayer::new().await.unwrap();
        
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        
        let request = RecordedRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers,
            body: Some("{\"name\":\"test\"}".to_string()),
        };
        
        let response = RecordedResponse {
            status_code: 201,
            headers: HashMap::new(),
            body: "{}".to_string(),
            duration_ms: 200,
        };
        
        let id = replayer.record(request, response);
        let curl = replayer.export_to_curl(&id).unwrap();
        
        assert!(curl.contains("curl"));
        assert!(curl.contains("POST"));
        assert!(curl.contains("Authorization"));
    }
}
