use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: RunStatus,
    pub profile: RunProfile,
    pub target: String,
    pub environment: super::EnvironmentTag,
    pub config: RunConfig,
    pub phases: HashMap<String, PhaseState>,
    pub findings_count: FindingsCount,
    pub metrics: RunMetrics,
    pub checkpoint: Option<Checkpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Initializing,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunProfile {
    Web,
    Api,
    Code,
    Full,
}

impl RunProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Api => "api",
            Self::Code => "code",
            Self::Full => "full",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "web" => Some(Self::Web),
            "api" => Some(Self::Api),
            "code" => Some(Self::Code),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub workers: u32,
    pub rate_limit: u32,
    pub timeout_seconds: u32,
    pub max_depth: u32,
    pub focus_classes: Vec<super::VulnClass>,
    pub dry_run: bool,
    pub no_exploit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub name: String,
    pub status: PhaseStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: f32,
    pub agent: String,
    pub tasks_completed: u32,
    pub tasks_total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingsCount {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub info: u32,
    pub confirmed: u32,
    pub unconfirmed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub requests_sent: u64,
    pub endpoints_discovered: u32,
    pub validations_attempted: u32,
    pub validations_successful: u32,
    pub memory_peak_mb: f32,
    pub cpu_peak_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub phase: String,
    pub state_data: serde_json::Value,
}

impl RunState {
    pub fn new(target: String, profile: RunProfile, environment: super::EnvironmentTag, config: RunConfig) -> Self {
        let now = Utc::now();
        
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            status: RunStatus::Initializing,
            profile,
            target,
            environment,
            config,
            phases: Self::initialize_phases(profile),
            findings_count: FindingsCount::default(),
            metrics: RunMetrics {
                start_time: now,
                end_time: None,
                duration_seconds: None,
                requests_sent: 0,
                endpoints_discovered: 0,
                validations_attempted: 0,
                validations_successful: 0,
                memory_peak_mb: 0.0,
                cpu_peak_percent: 0.0,
            },
            checkpoint: None,
        }
    }

    fn initialize_phases(profile: RunProfile) -> HashMap<String, PhaseState> {
        let phase_names = match profile {
            RunProfile::Web => vec!["scope", "recon", "auth", "surface_map", "hypothesis", "validation", "evidence", "report"],
            RunProfile::Api => vec!["scope", "recon", "auth", "surface_map", "hypothesis", "validation", "evidence", "report"],
            RunProfile::Code => vec!["scope", "code_analysis", "hypothesis", "validation", "evidence", "report"],
            RunProfile::Full => vec!["scope", "recon", "auth", "surface_map", "code_analysis", "hypothesis", "validation", "evidence", "root_cause", "remediation", "report"],
        };

        phase_names.into_iter().map(|name| {
            (name.to_string(), PhaseState {
                name: name.to_string(),
                status: PhaseStatus::Pending,
                started_at: None,
                completed_at: None,
                progress: 0.0,
                agent: format!("{}Agent", name),
                tasks_completed: 0,
                tasks_total: 0,
            })
        }).collect()
    }

    pub fn start_phase(&mut self, phase_name: &str) {
        if let Some(phase) = self.phases.get_mut(phase_name) {
            phase.status = PhaseStatus::Running;
            phase.started_at = Some(Utc::now());
        }
        self.updated_at = Utc::now();
    }

    pub fn complete_phase(&mut self, phase_name: &str) {
        if let Some(phase) = self.phases.get_mut(phase_name) {
            phase.status = PhaseStatus::Completed;
            phase.completed_at = Some(Utc::now());
            phase.progress = 1.0;
        }
        self.updated_at = Utc::now();
    }

    pub fn fail_phase(&mut self, phase_name: &str) {
        if let Some(phase) = self.phases.get_mut(phase_name) {
            phase.status = PhaseStatus::Failed;
        }
        self.status = RunStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn update_phase_progress(&mut self, phase_name: &str, progress: f32, completed: u32, total: u32) {
        if let Some(phase) = self.phases.get_mut(phase_name) {
            phase.progress = progress.clamp(0.0, 1.0);
            phase.tasks_completed = completed;
            phase.tasks_total = total;
        }
        self.updated_at = Utc::now();
    }

    pub fn create_checkpoint(&mut self, phase: String, state_data: serde_json::Value) {
        self.checkpoint = Some(Checkpoint {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            phase,
            state_data,
        });
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = RunStatus::Completed;
        self.metrics.end_time = Some(Utc::now());
        if let Some(duration) = self.metrics.end_time {
            self.metrics.duration_seconds = Some((duration - self.metrics.start_time).num_seconds() as u64);
        }
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = RunStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn overall_progress(&self) -> f32 {
        if self.phases.is_empty() {
            return 0.0;
        }

        let total_progress: f32 = self.phases.values().map(|p| p.progress).sum();
        total_progress / self.phases.len() as f32
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            workers: 16,
            rate_limit: 50,
            timeout_seconds: 300,
            max_depth: 10,
            focus_classes: Vec::new(),
            dry_run: false,
            no_exploit: false,
        }
    }
}
