use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct ScanDiff {
    pub added_endpoints: HashSet<String>,
    pub modified_endpoints: HashSet<String>,
    pub removed_endpoints: HashSet<String>,
}

impl ScanDiff {
    pub fn new() -> Self {
        Self {
            added_endpoints: HashSet::new(),
            modified_endpoints: HashSet::new(),
            removed_endpoints: HashSet::new(),
        }
    }
    
    pub fn add_endpoint(&mut self, endpoint: String) {
        self.added_endpoints.insert(endpoint);
    }
    
    pub fn modify_endpoint(&mut self, endpoint: String) {
        self.modified_endpoints.insert(endpoint);
    }
    
    pub fn remove_endpoint(&mut self, endpoint: String) {
        self.removed_endpoints.insert(endpoint);
    }
    
    pub fn total_changes(&self) -> usize {
        self.added_endpoints.len() + self.modified_endpoints.len() + self.removed_endpoints.len()
    }
    
    pub fn merge(mut self, other: ScanDiff) -> Self {
        self.added_endpoints.extend(other.added_endpoints);
        self.modified_endpoints.extend(other.modified_endpoints);
        self.removed_endpoints.extend(other.removed_endpoints);
        self
    }
}

impl Default for ScanDiff {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IncrementalScanner {
    priority_weights: PriorityWeights,
}

#[derive(Debug, Clone)]
struct PriorityWeights {
    added: u32,
    modified: u32,
    unchanged: u32,
}

impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            added: 100,
            modified: 50,
            unchanged: 10,
        }
    }
}

impl IncrementalScanner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            priority_weights: PriorityWeights::default(),
        })
    }
    
    pub fn compare_endpoints(&self, old: &[String], new: &[String]) -> ScanDiff {
        let old_set: HashSet<_> = old.iter().cloned().collect();
        let new_set: HashSet<_> = new.iter().cloned().collect();
        
        let mut diff = ScanDiff::new();
        
        for endpoint in &new_set {
            if !old_set.contains(endpoint) {
                diff.add_endpoint(endpoint.clone());
            }
        }
        
        for endpoint in &old_set {
            if !new_set.contains(endpoint) {
                diff.remove_endpoint(endpoint.clone());
            }
        }
        
        diff
    }
    
    pub fn calculate_priority_score(&self, change_type: ChangeType) -> u32 {
        match change_type {
            ChangeType::Added => self.priority_weights.added,
            ChangeType::Modified => self.priority_weights.modified,
            ChangeType::Unchanged => self.priority_weights.unchanged,
            ChangeType::Removed => 0,
        }
    }
    
    pub fn should_scan(&self, change_type: ChangeType) -> bool {
        matches!(change_type, ChangeType::Added | ChangeType::Modified)
    }
    
    pub fn create_batches(&self, endpoints: &[String], batch_size: usize) -> Vec<Vec<String>> {
        endpoints
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
    
    pub fn filter_by_priority(&self, diff: &ScanDiff, min_priority: u32) -> Vec<String> {
        let mut result = Vec::new();
        
        for endpoint in &diff.added_endpoints {
            if self.calculate_priority_score(ChangeType::Added) >= min_priority {
                result.push(endpoint.clone());
            }
        }
        
        for endpoint in &diff.modified_endpoints {
            if self.calculate_priority_score(ChangeType::Modified) >= min_priority {
                result.push(endpoint.clone());
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_diff_creation() {
        let diff = ScanDiff::new();
        assert_eq!(diff.total_changes(), 0);
    }
    
    #[test]
    fn test_incremental_scanner_creation() {
        let scanner = IncrementalScanner::new();
        assert!(scanner.is_ok());
    }
}
