use strike_security::tools::api_fuzzer::ApiFuzzer;
use serde_json::json;

#[test]
fn test_graphql_introspection_query_generation() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    assert!(!requests.is_empty());
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].url, "/graphql");
    assert!(requests[0].body.is_some());
    assert!(requests[0].body.as_ref().unwrap().contains("__schema"));
}

#[test]
fn test_graphql_query_fuzzing() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({
        "data": {
            "__schema": {
                "types": [
                    {
                        "name": "User",
                        "fields": [
                            {"name": "id"},
                            {"name": "name"},
                            {"name": "email"}
                        ]
                    }
                ]
            }
        }
    });
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let query_requests: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type.starts_with("query_"))
        .collect();
    
    assert!(!query_requests.is_empty());
}

#[test]
fn test_graphql_mutation_fuzzing() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({
        "data": {
            "__schema": {
                "types": [
                    {
                        "name": "User",
                        "fields": []
                    }
                ]
            }
        }
    });
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let mutation_requests: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type.starts_with("mutation_"))
        .collect();
    
    assert!(!mutation_requests.is_empty());
}

#[test]
fn test_graphql_batching_attacks() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let batching_attacks: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type.starts_with("batching_attack_"))
        .collect();
    
    assert!(!batching_attacks.is_empty());
    assert!(batching_attacks.len() >= 4);
}

#[test]
fn test_graphql_depth_attacks() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let depth_attacks: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type.starts_with("depth_attack_"))
        .collect();
    
    assert!(!depth_attacks.is_empty());
    
    for attack in depth_attacks {
        assert!(attack.body.is_some());
        let body = attack.body.as_ref().unwrap();
        assert!(body.contains("friends"));
    }
}

#[test]
fn test_graphql_directive_attacks() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let directive_attacks: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type == "directive_attack")
        .collect();
    
    assert!(!directive_attacks.is_empty());
    
    for attack in directive_attacks {
        assert!(attack.body.is_some());
        let body = attack.body.as_ref().unwrap();
        assert!(body.contains("@include") || body.contains("@skip") || body.contains("@deprecated"));
    }
}

#[test]
fn test_graphql_mutation_attacks() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({
        "data": {
            "__schema": {
                "mutationType": {
                    "fields": [
                        {"name": "createUser"},
                        {"name": "updateUser"}
                    ]
                }
            }
        }
    });
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let mutation_attacks: Vec<_> = requests.iter()
        .filter(|r| r.mutation_type.starts_with("mutation_attack_"))
        .collect();
    
    assert!(!mutation_attacks.is_empty());
}

#[test]
fn test_graphql_builtin_types_excluded() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({
        "data": {
            "__schema": {
                "types": [
                    {"name": "String", "fields": []},
                    {"name": "Int", "fields": []},
                    {"name": "User", "fields": [{"name": "id"}]}
                ]
            }
        }
    });
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let has_string_query = requests.iter()
        .any(|r| r.mutation_type == "query_String");
    let has_int_query = requests.iter()
        .any(|r| r.mutation_type == "query_Int");
    let has_user_query = requests.iter()
        .any(|r| r.mutation_type == "query_User");
    
    assert!(!has_string_query);
    assert!(!has_int_query);
    assert!(has_user_query);
}

#[test]
fn test_graphql_headers_content_type() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    for request in requests {
        assert!(request.headers.contains_key("Content-Type"));
        assert_eq!(request.headers.get("Content-Type").unwrap(), "application/json");
    }
}

#[test]
fn test_graphql_batching_sizes() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let batching_10 = requests.iter()
        .any(|r| r.mutation_type == "batching_attack_10");
    let batching_50 = requests.iter()
        .any(|r| r.mutation_type == "batching_attack_50");
    let batching_100 = requests.iter()
        .any(|r| r.mutation_type == "batching_attack_100");
    let batching_500 = requests.iter()
        .any(|r| r.mutation_type == "batching_attack_500");
    
    assert!(batching_10);
    assert!(batching_50);
    assert!(batching_100);
    assert!(batching_500);
}

#[test]
fn test_graphql_depth_levels() {
    let fuzzer = ApiFuzzer::new();
    let introspection = json!({});
    
    let requests = fuzzer.fuzz_from_graphql(&introspection);
    
    let depth_10 = requests.iter()
        .any(|r| r.mutation_type == "depth_attack_10");
    let depth_20 = requests.iter()
        .any(|r| r.mutation_type == "depth_attack_20");
    let depth_50 = requests.iter()
        .any(|r| r.mutation_type == "depth_attack_50");
    let depth_100 = requests.iter()
        .any(|r| r.mutation_type == "depth_attack_100");
    
    assert!(depth_10);
    assert!(depth_20);
    assert!(depth_50);
    assert!(depth_100);
}
