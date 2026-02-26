use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkflowPhase {
    ScopeValidation,
    Recon,
    AuthBootstrap,
    HypothesisGeneration,
    Validation,
    EvidenceCollection,
    RootCauseAnalysis,
    RemediationGeneration,
    ReportGeneration,
    HumanReview,
}

impl WorkflowPhase {
    pub fn name(&self) -> &str {
        match self {
            WorkflowPhase::ScopeValidation => "SCOPE_VALIDATION",
            WorkflowPhase::Recon => "RECON",
            WorkflowPhase::AuthBootstrap => "AUTH_BOOTSTRAP",
            WorkflowPhase::HypothesisGeneration => "HYPOTHESIS_GENERATION",
            WorkflowPhase::Validation => "VALIDATION",
            WorkflowPhase::EvidenceCollection => "EVIDENCE_COLLECTION",
            WorkflowPhase::RootCauseAnalysis => "ROOT_CAUSE_ANALYSIS",
            WorkflowPhase::RemediationGeneration => "REMEDIATION_GENERATION",
            WorkflowPhase::ReportGeneration => "REPORT_GENERATION",
            WorkflowPhase::HumanReview => "HUMAN_REVIEW",
        }
    }
    
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "SCOPE_VALIDATION" => Some(WorkflowPhase::ScopeValidation),
            "RECON" => Some(WorkflowPhase::Recon),
            "AUTH_BOOTSTRAP" => Some(WorkflowPhase::AuthBootstrap),
            "HYPOTHESIS_GENERATION" => Some(WorkflowPhase::HypothesisGeneration),
            "VALIDATION" => Some(WorkflowPhase::Validation),
            "EVIDENCE_COLLECTION" => Some(WorkflowPhase::EvidenceCollection),
            "ROOT_CAUSE_ANALYSIS" => Some(WorkflowPhase::RootCauseAnalysis),
            "REMEDIATION_GENERATION" => Some(WorkflowPhase::RemediationGeneration),
            "REPORT_GENERATION" => Some(WorkflowPhase::ReportGeneration),
            "HUMAN_REVIEW" => Some(WorkflowPhase::HumanReview),
            _ => None,
        }
    }
    
    pub fn all_phases() -> Vec<WorkflowPhase> {
        vec![
            WorkflowPhase::ScopeValidation,
            WorkflowPhase::Recon,
            WorkflowPhase::AuthBootstrap,
            WorkflowPhase::HypothesisGeneration,
            WorkflowPhase::Validation,
            WorkflowPhase::EvidenceCollection,
            WorkflowPhase::RootCauseAnalysis,
            WorkflowPhase::RemediationGeneration,
            WorkflowPhase::ReportGeneration,
            WorkflowPhase::HumanReview,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseConfig {
    pub phase: WorkflowPhase,
    pub agent: Option<String>,
    pub blocking: bool,
    pub parallel: bool,
    pub manual_gate: bool,
    pub dependencies: Vec<WorkflowPhase>,
}

impl PhaseConfig {
    pub fn new(phase: WorkflowPhase) -> Self {
        let (agent, blocking, parallel, manual_gate, dependencies) = match &phase {
            WorkflowPhase::ScopeValidation => {
                (Some("ScopeAgent".to_string()), true, false, false, vec![])
            }
            WorkflowPhase::Recon => {
                (Some("ReconAgent".to_string()), false, false, false, vec![WorkflowPhase::ScopeValidation])
            }
            WorkflowPhase::AuthBootstrap => {
                (Some("AuthAgent".to_string()), true, false, false, vec![WorkflowPhase::ScopeValidation])
            }
            WorkflowPhase::HypothesisGeneration => {
                (Some("HypothesisAgent".to_string()), false, false, false, vec![WorkflowPhase::Recon, WorkflowPhase::AuthBootstrap])
            }
            WorkflowPhase::Validation => {
                (Some("ValidationAgent".to_string()), false, true, false, vec![WorkflowPhase::HypothesisGeneration])
            }
            WorkflowPhase::EvidenceCollection => {
                (Some("EvidenceAgent".to_string()), false, false, false, vec![WorkflowPhase::Validation])
            }
            WorkflowPhase::RootCauseAnalysis => {
                (Some("RootCauseAgent".to_string()), false, false, false, vec![WorkflowPhase::EvidenceCollection])
            }
            WorkflowPhase::RemediationGeneration => {
                (Some("RemediationAgent".to_string()), false, false, false, vec![WorkflowPhase::RootCauseAnalysis])
            }
            WorkflowPhase::ReportGeneration => {
                (Some("ReportAgent".to_string()), true, false, false, vec![WorkflowPhase::RemediationGeneration])
            }
            WorkflowPhase::HumanReview => {
                (None, true, false, true, vec![WorkflowPhase::ReportGeneration])
            }
        };
        
        Self {
            phase,
            agent,
            blocking,
            parallel,
            manual_gate,
            dependencies,
        }
    }
    
    pub fn default_pipeline() -> Vec<PhaseConfig> {
        WorkflowPhase::all_phases()
            .into_iter()
            .map(PhaseConfig::new)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_name_conversion() {
        let phase = WorkflowPhase::ScopeValidation;
        assert_eq!(phase.name(), "SCOPE_VALIDATION");
        
        let parsed = WorkflowPhase::from_name("SCOPE_VALIDATION");
        assert_eq!(parsed, Some(WorkflowPhase::ScopeValidation));
    }
    
    #[test]
    fn test_all_phases() {
        let phases = WorkflowPhase::all_phases();
        assert_eq!(phases.len(), 10);
    }
    
    #[test]
    fn test_phase_config_creation() {
        let config = PhaseConfig::new(WorkflowPhase::Validation);
        assert_eq!(config.agent, Some("ValidationAgent".to_string()));
        assert!(config.parallel);
        assert!(!config.blocking);
    }
    
    #[test]
    fn test_default_pipeline() {
        let pipeline = PhaseConfig::default_pipeline();
        assert_eq!(pipeline.len(), 10);
        
        assert_eq!(pipeline[0].phase, WorkflowPhase::ScopeValidation);
        assert!(pipeline[0].blocking);
        
        assert_eq!(pipeline[4].phase, WorkflowPhase::Validation);
        assert!(pipeline[4].parallel);
    }
    
    #[test]
    fn test_phase_dependencies() {
        let config = PhaseConfig::new(WorkflowPhase::HypothesisGeneration);
        assert_eq!(config.dependencies.len(), 2);
        assert!(config.dependencies.contains(&WorkflowPhase::Recon));
        assert!(config.dependencies.contains(&WorkflowPhase::AuthBootstrap));
    }
}
