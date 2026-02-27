use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub mutation_type: String,
}

pub struct ApiFuzzer;

impl ApiFuzzer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn fuzz_from_openapi(&self, _schema: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        requests.push(FuzzRequest {
            method: "GET".to_string(),
            url: "/api/users".to_string(),
            headers: HashMap::new(),
            body: None,
            mutation_type: "baseline".to_string(),
        });
        
        requests
    }
    
    pub fn fuzz_from_graphql(&self, _introspection: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        requests.push(FuzzRequest {
            method: "POST".to_string(),
            url: "/graphql".to_string(),
            headers: HashMap::new(),
            body: Some("{\"query\":\"{ __schema { types { name } } }\"}".to_string()),
            mutation_type: "introspection".to_string(),
        });
        
        requests
    }
    
    pub fn generate_boundary_values(&self, field_type: &str) -> Vec<String> {
        match field_type {
            "integer" => vec![
                "0".to_string(),
                "-1".to_string(),
                "2147483647".to_string(),
                "-2147483648".to_string(),
                "9999999999".to_string(),
            ],
            "string" => vec![
                "".to_string(),
                "a".repeat(1000),
                "null".to_string(),
                "<script>alert(1)</script>".to_string(),
                "' OR '1'='1".to_string(),
            ],
            "boolean" => vec![
                "true".to_string(),
                "false".to_string(),
                "null".to_string(),
                "1".to_string(),
                "0".to_string(),
            ],
            "array" => vec![
                "[]".to_string(),
                "[null]".to_string(),
                format!("[{}]", "1,".repeat(1000)),
            ],
            _ => vec!["null".to_string()],
        }
    }
    
    pub fn generate_negative_tests(&self, endpoint: &str) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        requests.push(FuzzRequest {
            method: "GET".to_string(),
            url: endpoint.to_string(),
            headers: HashMap::new(),
            body: None,
            mutation_type: "missing_auth".to_string(),
        });
        
        requests.push(FuzzRequest {
            method: "POST".to_string(),
            url: endpoint.to_string(),
            headers: HashMap::new(),
            body: Some("invalid json".to_string()),
            mutation_type: "invalid_json".to_string(),
        });
        
        requests.push(FuzzRequest {
            method: "DELETE".to_string(),
            url: endpoint.to_string(),
            headers: HashMap::new(),
            body: None,
            mutation_type: "wrong_method".to_string(),
        });
        
        requests
    }
    
    pub fn fuzz_path_params(&self, template: &str) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let mutations = vec![
            "../etc/passwd",
            "..\\..\\windows\\system32",
            "%00",
            "null",
            "-1",
            "999999999",
            "<script>alert(1)</script>",
            "' OR '1'='1",
        ];
        
        for mutation in mutations {
            let url = template.replace("{id}", mutation);
            requests.push(FuzzRequest {
                method: "GET".to_string(),
                url,
                headers: HashMap::new(),
                body: None,
                mutation_type: format!("path_param_mutation: {}", mutation),
            });
        }
        
        requests
    }
}

impl Default for ApiFuzzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_fuzzer_creation() {
        let fuzzer = ApiFuzzer::new();
        assert_eq!(fuzzer, ApiFuzzer);
    }
    
    #[test]
    fn test_generate_boundary_values_integer() {
        let fuzzer = ApiFuzzer::new();
        let values = fuzzer.generate_boundary_values("integer");
        
        assert!(!values.is_empty());
        assert!(values.contains(&"0".to_string()));
        assert!(values.contains(&"-1".to_string()));
    }
    
    #[test]
    fn test_generate_boundary_values_string() {
        let fuzzer = ApiFuzzer::new();
        let values = fuzzer.generate_boundary_values("string");
        
        assert!(!values.is_empty());
        assert!(values.iter().any(|v| v.contains("script")));
    }
    
    #[test]
    fn test_generate_negative_tests() {
        let fuzzer = ApiFuzzer::new();
        let requests = fuzzer.generate_negative_tests("/api/users");
        
        assert_eq!(requests.len(), 3);
        assert!(requests.iter().any(|r| r.mutation_type == "missing_auth"));
        assert!(requests.iter().any(|r| r.mutation_type == "invalid_json"));
    }
    
    #[test]
    fn test_fuzz_path_params() {
        let fuzzer = ApiFuzzer::new();
        let requests = fuzzer.fuzz_path_params("/api/users/{id}");
        
        assert!(!requests.is_empty());
        assert!(requests.iter().any(|r| r.url.contains("etc/passwd")));
        assert!(requests.iter().any(|r| r.url.contains("script")));
    }
    
    #[test]
    fn test_fuzz_from_openapi() {
        let fuzzer = ApiFuzzer::new();
        let schema = serde_json::json!({});
        let requests = fuzzer.fuzz_from_openapi(&schema);
        
        assert!(!requests.is_empty());
    }
}
