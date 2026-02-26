use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvssV4Score {
    pub score: f32,
    pub vector: String,
    pub severity: CvssSeverity,
    pub base_metrics: BaseMetrics,
    pub threat_metrics: Option<ThreatMetrics>,
    pub environmental_metrics: Option<EnvironmentalMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMetrics {
    pub attack_vector: AttackVector,
    pub attack_complexity: AttackComplexity,
    pub attack_requirements: AttackRequirements,
    pub privileges_required: PrivilegesRequired,
    pub user_interaction: UserInteraction,
    pub confidentiality: Impact,
    pub integrity: Impact,
    pub availability: Impact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatMetrics {
    pub exploit_maturity: ExploitMaturity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalMetrics {
    pub confidentiality_requirement: Requirement,
    pub integrity_requirement: Requirement,
    pub availability_requirement: Requirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CvssSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl CvssSeverity {
    pub fn from_score(score: f32) -> Self {
        match score {
            0.0 => Self::None,
            0.1..=3.9 => Self::Low,
            4.0..=6.9 => Self::Medium,
            7.0..=8.9 => Self::High,
            9.0..=10.0 => Self::Critical,
            _ => Self::None,
        }
    }
}

impl fmt::Display for CvssSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackVector {
    Network,
    Adjacent,
    Local,
    Physical,
}

impl AttackVector {
    pub fn score(&self) -> f32 {
        match self {
            Self::Network => 0.85,
            Self::Adjacent => 0.62,
            Self::Local => 0.55,
            Self::Physical => 0.2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackComplexity {
    Low,
    High,
}

impl AttackComplexity {
    pub fn score(&self) -> f32 {
        match self {
            Self::Low => 0.77,
            Self::High => 0.44,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttackRequirements {
    None,
    Present,
}

impl AttackRequirements {
    pub fn score(&self) -> f32 {
        match self {
            Self::None => 0.85,
            Self::Present => 0.62,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrivilegesRequired {
    None,
    Low,
    High,
}

impl PrivilegesRequired {
    pub fn score(&self) -> f32 {
        match self {
            Self::None => 0.85,
            Self::Low => 0.62,
            Self::High => 0.27,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UserInteraction {
    None,
    Passive,
    Active,
}

impl UserInteraction {
    pub fn score(&self) -> f32 {
        match self {
            Self::None => 0.85,
            Self::Passive => 0.62,
            Self::Active => 0.45,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Impact {
    None,
    Low,
    High,
}

impl Impact {
    pub fn score(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Low => 0.22,
            Self::High => 0.56,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExploitMaturity {
    NotDefined,
    Attacked,
    ProofOfConcept,
    Unreported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Requirement {
    NotDefined,
    Low,
    Medium,
    High,
}

impl CvssV4Score {
    pub fn calculate(base: BaseMetrics) -> Self {
        let impact_score = (base.confidentiality.score() + base.integrity.score() + base.availability.score()).min(0.56);
        
        let exploitability = base.attack_vector.score() 
            * base.attack_complexity.score() 
            * base.attack_requirements.score()
            * base.privileges_required.score() 
            * base.user_interaction.score();

        let base_score = if impact_score == 0.0 {
            0.0
        } else {
            let score = (exploitability * impact_score * 10.0).min(10.0);
            (score * 10.0).round() / 10.0
        };

        let severity = CvssSeverity::from_score(base_score);
        let vector = Self::generate_vector(&base);

        Self {
            score: base_score,
            vector,
            severity,
            base_metrics: base,
            threat_metrics: None,
            environmental_metrics: None,
        }
    }

    fn generate_vector(base: &BaseMetrics) -> String {
        let av = match base.attack_vector {
            AttackVector::Network => "N",
            AttackVector::Adjacent => "A",
            AttackVector::Local => "L",
            AttackVector::Physical => "P",
        };

        let ac = match base.attack_complexity {
            AttackComplexity::Low => "L",
            AttackComplexity::High => "H",
        };

        let at = match base.attack_requirements {
            AttackRequirements::None => "N",
            AttackRequirements::Present => "P",
        };

        let pr = match base.privileges_required {
            PrivilegesRequired::None => "N",
            PrivilegesRequired::Low => "L",
            PrivilegesRequired::High => "H",
        };

        let ui = match base.user_interaction {
            UserInteraction::None => "N",
            UserInteraction::Passive => "P",
            UserInteraction::Active => "A",
        };

        let c = match base.confidentiality {
            Impact::None => "N",
            Impact::Low => "L",
            Impact::High => "H",
        };

        let i = match base.integrity {
            Impact::None => "N",
            Impact::Low => "L",
            Impact::High => "H",
        };

        let a = match base.availability {
            Impact::None => "N",
            Impact::Low => "L",
            Impact::High => "H",
        };

        format!("CVSS:4.0/AV:{}/AC:{}/AT:{}/PR:{}/UI:{}/VC:{}/VI:{}/VA:{}", av, ac, at, pr, ui, c, i, a)
    }
}
