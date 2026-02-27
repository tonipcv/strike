use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub header: Value,
    pub payload: Value,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtVulnerability {
    pub vuln_type: String,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpiryStatus {
    Valid,
    Expired,
    NotSet,
}

pub struct JwtInspector;

impl JwtInspector {
    pub fn new() -> Self {
        Self
    }
    
    pub fn decode(&self, token: &str) -> Result<JwtClaims> {
        let parts: Vec<&str> = token.split('.').collect();
        
        if parts.len() != 3 {
            anyhow::bail!("Invalid JWT format");
        }
        
        let header = self.decode_base64_json(parts[0])?;
        let payload = self.decode_base64_json(parts[1])?;
        let signature = parts[2].to_string();
        
        Ok(JwtClaims {
            header,
            payload,
            signature,
        })
    }
    
    fn decode_base64_json(&self, input: &str) -> Result<Value> {
        let padded = match input.len() % 4 {
            2 => format!("{}==", input),
            3 => format!("{}=", input),
            _ => input.to_string(),
        };
        
        let decoded = general_purpose::URL_SAFE_NO_PAD
            .decode(padded.as_bytes())
            .or_else(|_| general_purpose::STANDARD.decode(padded.as_bytes()))?;
        
        let json: Value = serde_json::from_slice(&decoded)?;
        Ok(json)
    }
    
    pub fn check_algorithm_confusion(&self, claims: &JwtClaims) -> Vec<JwtVulnerability> {
        let mut vulns = Vec::new();
        
        if let Some(alg) = claims.header.get("alg").and_then(|v| v.as_str()) {
            if alg == "none" {
                vulns.push(JwtVulnerability {
                    vuln_type: "AlgorithmNone".to_string(),
                    severity: "Critical".to_string(),
                    description: "JWT uses 'none' algorithm - signature verification disabled".to_string(),
                });
            }
            
            if alg.starts_with("HS") && claims.header.get("typ").is_some() {
                vulns.push(JwtVulnerability {
                    vuln_type: "AlgorithmConfusion".to_string(),
                    severity: "High".to_string(),
                    description: "Potential algorithm confusion vulnerability (HS vs RS)".to_string(),
                });
            }
        }
        
        vulns
    }
    
    pub fn check_none_algorithm(&self, claims: &JwtClaims) -> bool {
        claims.header
            .get("alg")
            .and_then(|v| v.as_str())
            .map(|alg| alg == "none")
            .unwrap_or(false)
    }
    
    pub fn check_weak_secret(&self, _token: &str, wordlist: &[String]) -> Option<String> {
        for word in wordlist {
            if word.len() < 8 {
                return Some(word.clone());
            }
        }
        None
    }
    
    pub fn check_expiry(&self, claims: &JwtClaims) -> ExpiryStatus {
        if let Some(exp) = claims.payload.get("exp").and_then(|v| v.as_i64()) {
            let now = chrono::Utc::now().timestamp();
            if exp < now {
                ExpiryStatus::Expired
            } else {
                ExpiryStatus::Valid
            }
        } else {
            ExpiryStatus::NotSet
        }
    }
    
    pub fn check_kid_injection(&self, claims: &JwtClaims) -> bool {
        if let Some(kid) = claims.header.get("kid").and_then(|v| v.as_str()) {
            kid.contains("../") || kid.contains("..\\") || kid.contains("/etc/")
        } else {
            false
        }
    }
}

impl Default for JwtInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_inspector_creation() {
        let inspector = JwtInspector::new();
        assert_eq!(inspector, JwtInspector);
    }
    
    #[test]
    fn test_decode_jwt() {
        let inspector = JwtInspector::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        
        let result = inspector.decode(token);
        assert!(result.is_ok());
        
        let claims = result.unwrap();
        assert_eq!(claims.header.get("alg").and_then(|v| v.as_str()), Some("HS256"));
        assert_eq!(claims.payload.get("name").and_then(|v| v.as_str()), Some("John Doe"));
    }
    
    #[test]
    fn test_check_none_algorithm() {
        let inspector = JwtInspector::new();
        let claims = JwtClaims {
            header: serde_json::json!({"alg": "none"}),
            payload: serde_json::json!({}),
            signature: "".to_string(),
        };
        
        assert!(inspector.check_none_algorithm(&claims));
    }
    
    #[test]
    fn test_check_kid_injection() {
        let inspector = JwtInspector::new();
        let claims = JwtClaims {
            header: serde_json::json!({"kid": "../../etc/passwd"}),
            payload: serde_json::json!({}),
            signature: "".to_string(),
        };
        
        assert!(inspector.check_kid_injection(&claims));
    }
    
    #[test]
    fn test_expiry_status() {
        let inspector = JwtInspector::new();
        
        let expired_claims = JwtClaims {
            header: serde_json::json!({}),
            payload: serde_json::json!({"exp": 1000000000}),
            signature: "".to_string(),
        };
        
        assert_eq!(inspector.check_expiry(&expired_claims), ExpiryStatus::Expired);
        
        let no_exp_claims = JwtClaims {
            header: serde_json::json!({}),
            payload: serde_json::json!({}),
            signature: "".to_string(),
        };
        
        assert_eq!(inspector.check_expiry(&no_exp_claims), ExpiryStatus::NotSet);
    }
}
