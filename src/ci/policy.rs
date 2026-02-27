use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiPolicy {
    pub fail_on: Severity,
    pub block_routes: Vec<String>,
    pub ignore_classes: Vec<String>,
    pub require_human_review_for: Option<Severity>,
    pub max_unreviewed_findings: Option<usize>,
    pub baseline_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for CiPolicy {
    fn default() -> Self {
        Self {
            fail_on: Severity::High,
            block_routes: vec![],
            ignore_classes: vec![],
            require_human_review_for: Some(Severity::Critical),
            max_unreviewed_findings: Some(10),
            baseline_file: Some(".strike/baseline.json".to_string()),
        }
    }
}

impl CiPolicy {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let policy: CiPolicy = toml::from_str(&content)?;
        Ok(policy)
    }
    
    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    
    pub fn should_fail(&self, severity: &Severity) -> bool {
        severity >= &self.fail_on
    }
    
    pub fn is_route_blocked(&self, route: &str) -> bool {
        self.block_routes.iter().any(|pattern| {
            route.contains(pattern) || Self::matches_pattern(route, pattern)
        })
    }
    
    pub fn is_class_ignored(&self, class: &str) -> bool {
        self.ignore_classes.contains(&class.to_string())
    }
    
    fn matches_pattern(route: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                route.starts_with(parts[0]) && route.ends_with(parts[1])
            } else {
                false
            }
        } else {
            route == pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_default() {
        let policy = CiPolicy::default();
        assert_eq!(policy.fail_on, Severity::High);
        assert!(policy.baseline_file.is_some());
    }
    
    #[test]
    fn test_should_fail() {
        let policy = CiPolicy::default();
        
        assert!(policy.should_fail(&Severity::Critical));
        assert!(policy.should_fail(&Severity::High));
        assert!(!policy.should_fail(&Severity::Medium));
        assert!(!policy.should_fail(&Severity::Low));
    }
    
    #[test]
    fn test_is_route_blocked() {
        let mut policy = CiPolicy::default();
        policy.block_routes = vec!["/api/admin".to_string(), "/internal/*".to_string()];
        
        assert!(policy.is_route_blocked("/api/admin"));
        assert!(policy.is_route_blocked("/internal/users"));
        assert!(!policy.is_route_blocked("/api/public"));
    }
    
    #[test]
    fn test_is_class_ignored() {
        let mut policy = CiPolicy::default();
        policy.ignore_classes = vec!["InfoDisclosure".to_string()];
        
        assert!(policy.is_class_ignored("InfoDisclosure"));
        assert!(!policy.is_class_ignored("SQLi"));
    }
    
    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }
}
