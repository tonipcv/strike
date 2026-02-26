use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub run_id: String,
    pub current_phase: Option<String>,
    pub phase_states: HashMap<String, PhaseState>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub phase_name: String,
    pub status: PhaseStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub attempts: u32,
    pub output: Option<serde_json::Value>,
}

impl WorkflowState {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            current_phase: None,
            phase_states: HashMap::new(),
            started_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn start_phase(&mut self, phase_name: String) {
        self.current_phase = Some(phase_name.clone());
        self.updated_at = Utc::now();
        
        let phase_state = PhaseState {
            phase_name: phase_name.clone(),
            status: PhaseStatus::Running,
            started_at: Some(Utc::now()),
            completed_at: None,
            error: None,
            attempts: self.phase_states.get(&phase_name)
                .map(|s| s.attempts + 1)
                .unwrap_or(1),
            output: None,
        };
        
        self.phase_states.insert(phase_name, phase_state);
    }
    
    pub fn complete_phase(&mut self, phase_name: String, output: Option<serde_json::Value>) {
        self.updated_at = Utc::now();
        
        if let Some(phase_state) = self.phase_states.get_mut(&phase_name) {
            phase_state.status = PhaseStatus::Completed;
            phase_state.completed_at = Some(Utc::now());
            phase_state.output = output;
        }
        
        self.current_phase = None;
    }
    
    pub fn fail_phase(&mut self, phase_name: String, error: String) {
        self.updated_at = Utc::now();
        
        if let Some(phase_state) = self.phase_states.get_mut(&phase_name) {
            phase_state.status = PhaseStatus::Failed;
            phase_state.completed_at = Some(Utc::now());
            phase_state.error = Some(error);
        }
        
        self.current_phase = None;
    }
    
    pub fn skip_phase(&mut self, phase_name: String) {
        self.updated_at = Utc::now();
        
        let phase_state = PhaseState {
            phase_name: phase_name.clone(),
            status: PhaseStatus::Skipped,
            started_at: None,
            completed_at: Some(Utc::now()),
            error: None,
            attempts: 0,
            output: None,
        };
        
        self.phase_states.insert(phase_name, phase_state);
    }
    
    pub fn is_phase_completed(&self, phase_name: &str) -> bool {
        self.phase_states
            .get(phase_name)
            .map(|s| s.status == PhaseStatus::Completed)
            .unwrap_or(false)
    }
    
    pub fn is_phase_failed(&self, phase_name: &str) -> bool {
        self.phase_states
            .get(phase_name)
            .map(|s| s.status == PhaseStatus::Failed)
            .unwrap_or(false)
    }
    
    pub fn get_completed_phases(&self) -> Vec<String> {
        self.phase_states
            .iter()
            .filter(|(_, state)| state.status == PhaseStatus::Completed)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    pub fn complete_run(&mut self) {
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_creation() {
        let state = WorkflowState::new("test-run".to_string());
        assert_eq!(state.run_id, "test-run");
        assert!(state.current_phase.is_none());
        assert!(state.completed_at.is_none());
    }
    
    #[test]
    fn test_phase_lifecycle() {
        let mut state = WorkflowState::new("test-run".to_string());
        
        state.start_phase("recon".to_string());
        assert_eq!(state.current_phase, Some("recon".to_string()));
        assert!(!state.is_phase_completed("recon"));
        
        state.complete_phase("recon".to_string(), None);
        assert!(state.is_phase_completed("recon"));
        assert!(state.current_phase.is_none());
    }
    
    #[test]
    fn test_phase_failure() {
        let mut state = WorkflowState::new("test-run".to_string());
        
        state.start_phase("validation".to_string());
        state.fail_phase("validation".to_string(), "Test error".to_string());
        
        assert!(state.is_phase_failed("validation"));
        assert!(state.current_phase.is_none());
    }
    
    #[test]
    fn test_get_completed_phases() {
        let mut state = WorkflowState::new("test-run".to_string());
        
        state.start_phase("recon".to_string());
        state.complete_phase("recon".to_string(), None);
        
        state.start_phase("auth".to_string());
        state.complete_phase("auth".to_string(), None);
        
        let completed = state.get_completed_phases();
        assert_eq!(completed.len(), 2);
        assert!(completed.contains(&"recon".to_string()));
        assert!(completed.contains(&"auth".to_string()));
    }
}
