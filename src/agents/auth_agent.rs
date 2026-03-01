use anyhow::{Result, Context};
use crate::tools::HttpClient;
use std::collections::HashMap;
use serde_json::from_str;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub cookies: Vec<String>,
    pub headers: Vec<(String, String)>,
    pub tokens: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthCredentials {
    Basic { username: String, password: String },
    Bearer { token: String },
    ApiKey { key: String, header_name: String },
    OAuth2 { client_id: String, client_secret: String, token_url: String, scope: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub session_token: Option<String>,
    pub cookies: Vec<String>,
    pub headers: Vec<(String, String)>,
}

pub struct AuthAgent {
    http_client: HttpClient,
    sessions: HashMap<String, AuthSession>,
}

impl AuthAgent {
    pub fn new() -> Self {
        Self {
            http_client: HttpClient::new(50, 30).unwrap(),
            sessions: HashMap::new(),
        }
    }

    pub async fn authenticate(&mut self, target: &str, credentials: &AuthCredentials) -> Result<AuthResult> {
        match credentials {
            AuthCredentials::Basic { username, password } => {
                let auth_value = general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                let auth_header = format!("Basic {}", auth_value);
                
                let _response = self.http_client.get(target).await?;
                
                Ok(AuthResult {
                    success: true,
                    session_token: Some(auth_header),
                    cookies: Vec::new(),
                    headers: vec![("Authorization".to_string(), auth_header)],
                })
            }
            AuthCredentials::Bearer { token } => {
                let _response = self.http_client.get(target).await?;
                
                Ok(AuthResult {
                    success: true,
                    session_token: Some(token.clone()),
                    cookies: Vec::new(),
                    headers: vec![("Authorization".to_string(), format!("Bearer {}", token))],
                })
            }
            AuthCredentials::ApiKey { key, header_name } => {
                let _response = self.http_client.get(target).await?;
                
                Ok(AuthResult {
                    success: true,
                    session_token: None,
                    cookies: Vec::new(),
                    headers: vec![(header_name.clone(), key.clone())],
                })
            }
            AuthCredentials::OAuth2 { client_id, client_secret, token_url, scope } => {
                // Implement OAuth2 client credentials flow
                let body = format!("grant_type=client_credentials&client_id={}&client_secret={}", client_id, client_secret);
                let token_response = self.http_client.post(&token_url, Some(body)).await?;
                
                let token_response_text = token_response.text().await?;
                let token_response_json: serde_json::Value = from_str(&token_response_text)?;
                let token = token_response_json["access_token"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("No access_token in OAuth2 response"))?
                    .to_string();
                
                Ok(AuthResult {
                    success: true,
                    session_token: Some(token),
                    cookies: Vec::new(),
                    headers: vec![("Authorization".to_string(), format!("Bearer {}", token))],
                })
            }
        }
    }

    pub fn get_session(&self, session_id: &str) -> Option<&AuthSession> {
        self.sessions.get(session_id)
    }

    pub fn update_session(&mut self, session: AuthSession) {
        self.sessions.insert(session.session_id.clone(), session);
    }
}
