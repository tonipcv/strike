use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use super::{VulnClass, CvssV4Score, Evidence, Target};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: Uuid,
    pub run_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub vuln_class: VulnClass,
    pub severity: Severity,
    pub cvss_v4_score: CvssV4Score,
    pub status: FindingStatus,
    pub target: Target,
    pub evidence: Evidence,
    pub root_cause: Option<RootCause>,
    pub remediation: Remediation,
    pub environment: Environment,
    pub authorization: Authorization,
    pub retest_history: Vec<RetestRecord>,
    pub human_review: Option<HumanReview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn from_cvss_score(score: f32) -> Self {
        match score {
            0.0 => Self::Info,
            0.1..=3.9 => Self::Low,
            4.0..=6.9 => Self::Medium,
            7.0..=8.9 => Self::High,
            9.0..=10.0 => Self::Critical,
            _ => Self::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Confirmed,
    Unconfirmed,
    NeedsReview,
    Fixed,
    WontFix,
}

impl FindingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unconfirmed => "unconfirmed",
            Self::NeedsReview => "needs_review",
            Self::Fixed => "fixed",
            Self::WontFix => "wont_fix",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    pub code_file: Option<String>,
    pub code_line: Option<u32>,
    pub pattern: String,
    pub asvs_control: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remediation {
    pub summary: String,
    pub code_diff: Option<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub tag: EnvironmentTag,
    pub target_build_sha: Option<String>,
    pub strike_version: String,
    pub run_config_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentTag {
    Staging,
    Sandbox,
    Local,
    Production,
}

impl EnvironmentTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Staging => "staging",
            Self::Sandbox => "sandbox",
            Self::Local => "local",
            Self::Production => "production",
        }
    }

    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    pub roe_reference: String,
    pub authorized_by: String,
    pub authorized_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestRecord {
    pub retested_at: DateTime<Utc>,
    pub result: RetestResult,
    pub run_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetestResult {
    StillVulnerable,
    Fixed,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReview {
    pub reviewed_by: String,
    pub reviewed_at: DateTime<Utc>,
    pub verdict: ReviewVerdict,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewVerdict {
    Accepted,
    Rejected,
    Modified,
}

impl Finding {
    pub fn new(
        run_id: Uuid,
        title: String,
        vuln_class: VulnClass,
        cvss_v4_score: CvssV4Score,
        target: Target,
        evidence: Evidence,
        environment: Environment,
        authorization: Authorization,
    ) -> Self {
        let severity = Severity::from_cvss_score(cvss_v4_score.score);
        
        Self {
            id: Uuid::new_v4(),
            run_id,
            timestamp: Utc::now(),
            title,
            vuln_class,
            severity,
            cvss_v4_score,
            status: FindingStatus::Unconfirmed,
            target,
            evidence,
            root_cause: None,
            remediation: Remediation {
                summary: String::new(),
                code_diff: None,
                references: Vec::new(),
            },
            environment,
            authorization,
            retest_history: Vec::new(),
            human_review: None,
        }
    }

    pub fn confirm(&mut self) {
        self.status = FindingStatus::Confirmed;
    }

    pub fn add_retest(&mut self, result: RetestResult, run_id: Uuid) {
        self.retest_history.push(RetestRecord {
            retested_at: Utc::now(),
            result,
            run_id,
        });

        if result == RetestResult::Fixed {
            self.status = FindingStatus::Fixed;
        }
    }

    pub fn add_human_review(&mut self, reviewed_by: String, verdict: ReviewVerdict, notes: Option<String>) {
        self.human_review = Some(HumanReview {
            reviewed_by,
            reviewed_at: Utc::now(),
            verdict,
            notes,
        });

        match verdict {
            ReviewVerdict::Accepted => {
                if self.status == FindingStatus::Unconfirmed {
                    self.status = FindingStatus::Confirmed;
                }
            }
            ReviewVerdict::Rejected => {
                self.status = FindingStatus::WontFix;
            }
            ReviewVerdict::Modified => {}
        }
    }
}
