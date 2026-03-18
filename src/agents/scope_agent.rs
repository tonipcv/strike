use anyhow::Result;
use crate::models::{RulesOfEngagement, ActionType};

pub struct ScopeAgent {
    roe: RulesOfEngagement,
}

impl ScopeAgent {
    pub fn new(roe: RulesOfEngagement) -> Self {
        Self { roe }
    }

    pub fn validate_target(&self, target: &str) -> Result<bool> {
        if !self.roe.is_target_in_scope(target) {
            return Err(anyhow::anyhow!(
                "Target '{}' is not in scope. Authorized targets: {:?}",
                target,
                self.roe.scope.targets
            ));
        }

        if self.roe.scope.environment.is_production() {
            return Err(anyhow::anyhow!(
                "Production environment detected. Use --force-prod flag with explicit justification."
            ));
        }

        Ok(true)
    }

    pub fn validate_action(&self, action: ActionType) -> Result<bool> {
        if !self.roe.is_action_allowed(action) {
            return Err(anyhow::anyhow!(
                "Action {:?} is not allowed by ROE",
                action
            ));
        }

        Ok(true)
    }

    pub fn requires_confirmation(&self, action: ActionType) -> bool {
        self.roe.requires_confirmation(action)
    }

    pub fn validate_rate(&self, current_rate: u32) -> Result<bool> {
        if !self.roe.validate_rate_limit(current_rate) {
            return Err(anyhow::anyhow!(
                "Rate limit exceeded. Current: {}, Max: {:?}",
                current_rate,
                self.roe.constraints.max_rate_per_second
            ));
        }

        Ok(true)
    }
    
    pub async fn analyze_scope(&self, target: &str) -> Result<Vec<String>> {
        self.validate_target(target)?;
        
        let mut scope_items = Vec::new();
        
        for authorized_target in &self.roe.scope.targets {
            scope_items.push(authorized_target.clone());
        }
        
        Ok(scope_items)
    }
}
