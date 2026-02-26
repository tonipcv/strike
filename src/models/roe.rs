use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesOfEngagement {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub authorized_by: String,
    pub scope: Scope,
    pub constraints: Constraints,
    pub approved_actions: HashSet<ActionType>,
    pub forbidden_actions: HashSet<ActionType>,
    pub contact_info: ContactInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub targets: Vec<String>,
    pub excluded_targets: Vec<String>,
    pub ip_ranges: Vec<String>,
    pub domains: Vec<String>,
    pub environment: super::EnvironmentTag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub max_rate_per_second: Option<u32>,
    pub max_concurrent_requests: Option<u32>,
    pub allowed_hours: Option<TimeWindow>,
    pub max_depth: Option<u32>,
    pub require_confirmation_for: Vec<ActionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Reconnaissance,
    PortScanning,
    VulnerabilityScanning,
    ExploitValidation,
    AuthenticationTesting,
    SqlInjectionTesting,
    XssTesting,
    SsrfTesting,
    FileUploadTesting,
    BruteForce,
    DenialOfService,
    DataExfiltration,
    PersistentAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub primary_contact: String,
    pub email: String,
    pub phone: Option<String>,
    pub emergency_contact: Option<String>,
}

impl RulesOfEngagement {
    pub fn is_target_in_scope(&self, target: &str) -> bool {
        for excluded in &self.scope.excluded_targets {
            if target.contains(excluded) {
                return false;
            }
        }

        if self.scope.targets.is_empty() {
            return true;
        }

        self.scope.targets.iter().any(|t| target.contains(t))
    }

    pub fn is_action_allowed(&self, action: ActionType) -> bool {
        if self.forbidden_actions.contains(&action) {
            return false;
        }

        if self.approved_actions.is_empty() {
            return true;
        }

        self.approved_actions.contains(&action)
    }

    pub fn requires_confirmation(&self, action: ActionType) -> bool {
        self.constraints.require_confirmation_for.contains(&action)
    }

    pub fn validate_rate_limit(&self, current_rate: u32) -> bool {
        match self.constraints.max_rate_per_second {
            Some(max) => current_rate <= max,
            None => true,
        }
    }
}

impl Default for RulesOfEngagement {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            authorized_by: String::new(),
            scope: Scope {
                targets: Vec::new(),
                excluded_targets: Vec::new(),
                ip_ranges: Vec::new(),
                domains: Vec::new(),
                environment: super::EnvironmentTag::Local,
            },
            constraints: Constraints {
                max_rate_per_second: Some(50),
                max_concurrent_requests: Some(16),
                allowed_hours: None,
                max_depth: Some(10),
                require_confirmation_for: vec![
                    ActionType::ExploitValidation,
                    ActionType::BruteForce,
                ],
            },
            approved_actions: HashSet::from([
                ActionType::Reconnaissance,
                ActionType::PortScanning,
                ActionType::VulnerabilityScanning,
                ActionType::ExploitValidation,
                ActionType::AuthenticationTesting,
            ]),
            forbidden_actions: HashSet::from([
                ActionType::DenialOfService,
                ActionType::DataExfiltration,
                ActionType::PersistentAccess,
            ]),
            contact_info: ContactInfo {
                primary_contact: String::new(),
                email: String::new(),
                phone: None,
                emergency_contact: None,
            },
        }
    }
}
