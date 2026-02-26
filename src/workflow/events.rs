use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowEvent {
    RunStarted {
        run_id: String,
        timestamp: DateTime<Utc>,
    },
    PhaseStarted {
        run_id: String,
        phase: String,
        timestamp: DateTime<Utc>,
    },
    PhaseCompleted {
        run_id: String,
        phase: String,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    PhaseFailed {
        run_id: String,
        phase: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    HypothesisGenerated {
        run_id: String,
        count: usize,
        timestamp: DateTime<Utc>,
    },
    ValidationStarted {
        run_id: String,
        hypothesis_id: String,
        timestamp: DateTime<Utc>,
    },
    FindingConfirmed {
        run_id: String,
        finding_id: String,
        severity: String,
        timestamp: DateTime<Utc>,
    },
    FindingDiscarded {
        run_id: String,
        hypothesis_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    EvidenceCaptured {
        run_id: String,
        finding_id: String,
        timestamp: DateTime<Utc>,
    },
    ReportGenerated {
        run_id: String,
        format: String,
        timestamp: DateTime<Utc>,
    },
    RunCompleted {
        run_id: String,
        total_findings: usize,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    RunFailed {
        run_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    CheckpointSaved {
        run_id: String,
        checkpoint_id: String,
        phase: String,
        timestamp: DateTime<Utc>,
    },
}

impl WorkflowEvent {
    pub fn id(&self) -> String {
        Uuid::new_v4().to_string()
    }
    
    pub fn run_id(&self) -> &str {
        match self {
            WorkflowEvent::RunStarted { run_id, .. } => run_id,
            WorkflowEvent::PhaseStarted { run_id, .. } => run_id,
            WorkflowEvent::PhaseCompleted { run_id, .. } => run_id,
            WorkflowEvent::PhaseFailed { run_id, .. } => run_id,
            WorkflowEvent::HypothesisGenerated { run_id, .. } => run_id,
            WorkflowEvent::ValidationStarted { run_id, .. } => run_id,
            WorkflowEvent::FindingConfirmed { run_id, .. } => run_id,
            WorkflowEvent::FindingDiscarded { run_id, .. } => run_id,
            WorkflowEvent::EvidenceCaptured { run_id, .. } => run_id,
            WorkflowEvent::ReportGenerated { run_id, .. } => run_id,
            WorkflowEvent::RunCompleted { run_id, .. } => run_id,
            WorkflowEvent::RunFailed { run_id, .. } => run_id,
            WorkflowEvent::CheckpointSaved { run_id, .. } => run_id,
        }
    }
    
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            WorkflowEvent::RunStarted { timestamp, .. } => timestamp,
            WorkflowEvent::PhaseStarted { timestamp, .. } => timestamp,
            WorkflowEvent::PhaseCompleted { timestamp, .. } => timestamp,
            WorkflowEvent::PhaseFailed { timestamp, .. } => timestamp,
            WorkflowEvent::HypothesisGenerated { timestamp, .. } => timestamp,
            WorkflowEvent::ValidationStarted { timestamp, .. } => timestamp,
            WorkflowEvent::FindingConfirmed { timestamp, .. } => timestamp,
            WorkflowEvent::FindingDiscarded { timestamp, .. } => timestamp,
            WorkflowEvent::EvidenceCaptured { timestamp, .. } => timestamp,
            WorkflowEvent::ReportGenerated { timestamp, .. } => timestamp,
            WorkflowEvent::RunCompleted { timestamp, .. } => timestamp,
            WorkflowEvent::RunFailed { timestamp, .. } => timestamp,
            WorkflowEvent::CheckpointSaved { timestamp, .. } => timestamp,
        }
    }
}

pub struct EventBus {
    sender: broadcast::Sender<WorkflowEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    
    pub fn publish(&self, event: WorkflowEvent) {
        let _ = self.sender.send(event);
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<WorkflowEvent> {
        self.sender.subscribe()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new(100);
        let _receiver = bus.subscribe();
    }
    
    #[tokio::test]
    async fn test_event_publishing() {
        let bus = EventBus::new(100);
        let mut receiver = bus.subscribe();
        
        let event = WorkflowEvent::RunStarted {
            run_id: "test-run".to_string(),
            timestamp: Utc::now(),
        };
        
        bus.publish(event.clone());
        
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.run_id(), "test-run");
    }
}
