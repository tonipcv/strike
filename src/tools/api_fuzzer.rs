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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    
    pub fn fuzz_from_graphql(&self, introspection: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        requests.push(FuzzRequest {
            method: "POST".to_string(),
            url: "/graphql".to_string(),
            headers: self.graphql_headers(),
            body: Some(self.introspection_query()),
            mutation_type: "introspection".to_string(),
        });
        
        if let Some(schema) = introspection.get("data").and_then(|d| d.get("__schema")) {
            if let Some(types) = schema.get("types").and_then(|t| t.as_array()) {
                for type_obj in types {
                    if let Some(type_name) = type_obj.get("name").and_then(|n| n.as_str()) {
                        if !type_name.starts_with("__") && !self.is_builtin_type(type_name) {
                            requests.extend(self.generate_query_fuzzing(type_name, type_obj));
                            requests.extend(self.generate_mutation_fuzzing(type_name, type_obj));
                        }
                    }
                }
            }
            
            if let Some(mutation_type) = schema.get("mutationType") {
                if let Some(fields) = mutation_type.get("fields").and_then(|f| f.as_array()) {
                    for field in fields {
                        if let Some(field_name) = field.get("name").and_then(|n| n.as_str()) {
                            requests.extend(self.generate_mutation_attack(field_name, field));
                        }
                    }
                }
            }
        }
        
        requests.extend(self.generate_batching_attacks());
        requests.extend(self.generate_depth_attacks());
        requests.extend(self.generate_directive_attacks());
        
        requests
    }
    
    fn introspection_query(&self) -> String {
        r#"{
            "query": "query IntrospectionQuery {
                __schema {
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                    types {
                        ...FullType
                    }
                    directives {
                        name
                        description
                        locations
                        args {
                            ...InputValue
                        }
                    }
                }
            }
            fragment FullType on __Type {
                kind
                name
                description
                fields(includeDeprecated: true) {
                    name
                    description
                    args {
                        ...InputValue
                    }
                    type {
                        ...TypeRef
                    }
                    isDeprecated
                    deprecationReason
                }
                inputFields {
                    ...InputValue
                }
                interfaces {
                    ...TypeRef
                }
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
                possibleTypes {
                    ...TypeRef
                }
            }
            fragment InputValue on __InputValue {
                name
                description
                type { ...TypeRef }
                defaultValue
            }
            fragment TypeRef on __Type {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                    ofType {
                                        kind
                                        name
                                        ofType {
                                            kind
                                            name
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }"
        }"#.to_string()
    }
    
    fn graphql_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }
    
    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(type_name, "String" | "Int" | "Float" | "Boolean" | "ID")
    }
    
    fn generate_query_fuzzing(&self, type_name: &str, type_obj: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        if let Some(fields) = type_obj.get("fields").and_then(|f| f.as_array()) {
            for field in fields.iter().take(5) {
                if let Some(field_name) = field.get("name").and_then(|n| n.as_str()) {
                    let query = format!(
                        r#"{{"query": "{{ {} {{ {} }} }}"}}"#,
                        type_name.to_lowercase(),
                        field_name
                    );
                    
                    requests.push(FuzzRequest {
                        method: "POST".to_string(),
                        url: "/graphql".to_string(),
                        headers: self.graphql_headers(),
                        body: Some(query),
                        mutation_type: format!("query_{}", type_name),
                    });
                }
            }
        }
        
        requests
    }
    
    fn generate_mutation_fuzzing(&self, type_name: &str, _type_obj: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let mutation_names = vec![
            format!("create{}", type_name),
            format!("update{}", type_name),
            format!("delete{}", type_name),
        ];
        
        for mutation_name in mutation_names {
            let query = format!(
                r#"{{"query": "mutation {{ {}(input: {{}}) {{ id }} }}"}}"#,
                mutation_name
            );
            
            requests.push(FuzzRequest {
                method: "POST".to_string(),
                url: "/graphql".to_string(),
                headers: self.graphql_headers(),
                body: Some(query),
                mutation_type: format!("mutation_{}", mutation_name),
            });
        }
        
        requests
    }
    
    fn generate_mutation_attack(&self, field_name: &str, field: &Value) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let malicious_inputs = vec![
            r#"{"id": "' OR '1'='1"}"#,
            r#"{"id": "../../../etc/passwd"}"#,
            r#"{"id": "<script>alert(1)</script>"}"#,
            r#"{"id": null}"#,
        ];
        
        for input in malicious_inputs {
            let query = format!(
                r#"{{"query": "mutation {{ {}(input: {}) {{ id }} }}"}}"#,
                field_name, input
            );
            
            requests.push(FuzzRequest {
                method: "POST".to_string(),
                url: "/graphql".to_string(),
                headers: self.graphql_headers(),
                body: Some(query),
                mutation_type: format!("mutation_attack_{}", field_name),
            });
        }
        
        requests
    }
    
    fn generate_batching_attacks(&self) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let batch_sizes = vec![10, 50, 100, 500];
        
        for size in batch_sizes {
            let queries: Vec<String> = (0..size)
                .map(|i| format!(r#"{{"query": "{{ user(id: {}) {{ id name }} }}"}}"#, i))
                .collect();
            
            let batch_body = format!("[{}]", queries.join(","));
            
            requests.push(FuzzRequest {
                method: "POST".to_string(),
                url: "/graphql".to_string(),
                headers: self.graphql_headers(),
                body: Some(batch_body),
                mutation_type: format!("batching_attack_{}", size),
            });
        }
        
        requests
    }
    
    fn generate_depth_attacks(&self) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let depths = vec![10, 20, 50, 100];
        
        for depth in depths {
            let mut query = String::from("{ user { ");
            for _ in 0..depth {
                query.push_str("friends { ");
            }
            query.push_str("id ");
            for _ in 0..depth {
                query.push_str("} ");
            }
            query.push_str("} }");
            
            let body = format!(r#"{{"query": "{}"}}"#, query);
            
            requests.push(FuzzRequest {
                method: "POST".to_string(),
                url: "/graphql".to_string(),
                headers: self.graphql_headers(),
                body: Some(body),
                mutation_type: format!("depth_attack_{}", depth),
            });
        }
        
        requests
    }
    
    fn generate_directive_attacks(&self) -> Vec<FuzzRequest> {
        let mut requests = Vec::new();
        
        let directive_attacks = vec![
            r#"{ user @include(if: true) { id } }"#,
            r#"{ user @skip(if: false) { id } }"#,
            r#"{ user @deprecated(reason: "test") { id } }"#,
        ];
        
        for attack in directive_attacks {
            let body = format!(r#"{{"query": "{}"}}"#, attack);
            
            requests.push(FuzzRequest {
                method: "POST".to_string(),
                url: "/graphql".to_string(),
                headers: self.graphql_headers(),
                body: Some(body),
                mutation_type: "directive_attack".to_string(),
            });
        }
        
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
