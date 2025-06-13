use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// The event sent by a service to indicate completion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletedTask {
    pub invocation_id: String,
    pub task_id: String,
    pub payload: serde_json::Value,
}

impl Into<WorkflowTask> for CompletedTask {
    fn into(self) -> WorkflowTask {
        WorkflowTask {
            invocation_id: self.invocation_id,
            task_id: self.task_id,
            state: WorkflowTaskState::Completed(self.payload),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
// Internal type discriminator
#[serde(tag = "type")]
pub struct WorkflowTask {
    pub invocation_id: String,
    pub task_id: String,

    // _expiry_timestamp: u64,
    // _in_progress_expiry_timestamp: u64,
    pub state: WorkflowTaskState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkflowTaskState {
    Running,
    Completed(serde_json::Value),
}
