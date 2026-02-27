use anyhow::{Context, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Topo;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use std::sync::Arc;

use super::phases::{PhaseConfig, WorkflowPhase};

pub struct AgentGraph {
    graph: DiGraph<WorkflowPhase, ()>,
    node_map: HashMap<WorkflowPhase, NodeIndex>,
}

impl AgentGraph {
    pub fn new(pipeline: &[PhaseConfig]) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        for phase_config in pipeline {
            let node = graph.add_node(phase_config.phase.clone());
            node_map.insert(phase_config.phase.clone(), node);
        }
        
        for phase_config in pipeline {
            let current_node = node_map[&phase_config.phase];
            
            for dependency in &phase_config.dependencies {
                if let Some(&dep_node) = node_map.get(dependency) {
                    graph.add_edge(dep_node, current_node, ());
                }
            }
        }
        
        Ok(Self { graph, node_map })
    }
    
    pub fn get_execution_order(&self) -> Vec<WorkflowPhase> {
        let mut topo = Topo::new(&self.graph);
        let mut order = Vec::new();
        
        while let Some(node) = topo.next(&self.graph) {
            if let Some(phase) = self.graph.node_weight(node) {
                order.push(phase.clone());
            }
        }
        
        order
    }
    
    pub fn get_parallel_groups(&self) -> Vec<Vec<WorkflowPhase>> {
        let execution_order = self.get_execution_order();
        let mut groups = Vec::new();
        let mut processed = std::collections::HashSet::new();
        
        for phase in &execution_order {
            if processed.contains(phase) {
                continue;
            }
            
            let mut group = vec![phase.clone()];
            processed.insert(phase.clone());
            
            for other_phase in &execution_order {
                if processed.contains(other_phase) {
                    continue;
                }
                
                if self.can_run_parallel(phase, other_phase) {
                    group.push(other_phase.clone());
                    processed.insert(other_phase.clone());
                }
            }
            
            groups.push(group);
        }
        
        groups
    }
    
    fn can_run_parallel(&self, phase1: &WorkflowPhase, phase2: &WorkflowPhase) -> bool {
        let node1 = self.node_map.get(phase1);
        let node2 = self.node_map.get(phase2);
        
        if node1.is_none() || node2.is_none() {
            return false;
        }
        
        let node1 = *node1.unwrap();
        let node2 = *node2.unwrap();
        
        !petgraph::algo::has_path_connecting(&self.graph, node1, node2, None) &&
        !petgraph::algo::has_path_connecting(&self.graph, node2, node1, None)
    }
    
    pub fn validate_acyclic(&self) -> Result<()> {
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            anyhow::bail!("Workflow graph contains cycles");
        }
        Ok(())
    }
}

pub struct ParallelScheduler {
    max_workers: usize,
}

impl ParallelScheduler {
    pub fn new(max_workers: usize) -> Self {
        Self {
            max_workers: max_workers.min(64),
        }
    }
    
    pub async fn execute_parallel<F, Fut, T>(
        &self,
        tasks: Vec<F>,
    ) -> Vec<Result<T>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(self.max_workers));
        let mut join_set = JoinSet::new();
        
        for task in tasks {
            let sem = semaphore.clone();
            
            join_set.spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                task().await
            });
        }
        
        let mut results = Vec::new();
        
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(task_result) => results.push(task_result),
                Err(e) => results.push(Err(anyhow::anyhow!("Task join error: {}", e))),
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::phases::PhaseConfig;

    #[test]
    fn test_agent_graph_creation() {
        let pipeline = PhaseConfig::default_pipeline();
        let graph = AgentGraph::new(&pipeline);
        assert!(graph.is_ok());
    }
    
    #[test]
    fn test_execution_order() {
        let pipeline = PhaseConfig::default_pipeline();
        let graph = AgentGraph::new(&pipeline).unwrap();
        
        let order = graph.get_execution_order();
        assert_eq!(order.len(), 10);
        
        assert_eq!(order[0], WorkflowPhase::ScopeValidation);
    }
    
    #[test]
    fn test_parallel_groups() {
        let pipeline = PhaseConfig::default_pipeline();
        let graph = AgentGraph::new(&pipeline).unwrap();
        
        let groups = graph.get_parallel_groups();
        assert!(!groups.is_empty());
        
        let second_group = &groups[1];
        assert!(second_group.contains(&WorkflowPhase::Recon) || 
                second_group.contains(&WorkflowPhase::AuthBootstrap));
    }
    
    #[test]
    fn test_graph_is_acyclic() {
        let pipeline = PhaseConfig::default_pipeline();
        let graph = AgentGraph::new(&pipeline).unwrap();
        
        let result = graph.validate_acyclic();
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_parallel_scheduler() {
        let scheduler = ParallelScheduler::new(4);
        
        let tasks: Vec<_> = (0..10)
            .map(|i| {
                move || async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    Ok::<_, anyhow::Error>(i * 2)
                }
            })
            .collect();
        
        let results = scheduler.execute_parallel(tasks).await;
        assert_eq!(results.len(), 10);
        
        for result in results {
            assert!(result.is_ok());
        }
    }
}
