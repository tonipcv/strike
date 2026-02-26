use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub tokens: HashMap<String, String>,
}

pub struct AuthAgent {
    sessions: HashMap<String, AuthSession>,
}

impl AuthAgent {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn authenticate_form(
        &mut self,
        target: &str,
        username: &str,
        password: &str,
    ) -> Result<AuthSession> {
        let session = AuthSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            cookies: HashMap::new(),
            headers: HashMap::new(),
            tokens: HashMap::new(),
        };

        self.sessions.insert(session.session_id.clone(), session.clone());

        Ok(session)
    }

    pub async fn authenticate_api_key(
        &mut self,
        target: &str,
        api_key: &str,
    ) -> Result<AuthSession> {
        let mut session = AuthSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            cookies: HashMap::new(),
            headers: HashMap::new(),
            tokens: HashMap::new(),
        };

        session.headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));

        self.sessions.insert(session.session_id.clone(), session.clone());

        Ok(session)
    }

    pub fn get_session(&self, session_id: &str) -> Option<&AuthSession> {
        self.sessions.get(session_id)
    }

    pub fn update_session(&mut self, session: AuthSession) {
        self.sessions.insert(session.session_id.clone(), session);
    }
}
