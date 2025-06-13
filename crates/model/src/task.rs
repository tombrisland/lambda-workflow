use crate::InvocationId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Deserialize, Clone)]
pub struct RunningTask {
    pub invocation_id: String,
    pub task_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletedTask {
    pub invocation_id: String,
    pub task_id: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub enum WorkflowTask {
    Running(RunningTask),
    Completed(CompletedTask),
}

impl InvocationId for WorkflowTask {
    fn invocation_id(&self) -> &str {
        match self {
            WorkflowTask::Running(inner) => &inner.invocation_id,
            WorkflowTask::Completed(inner) => &inner.invocation_id,
        }
    }
}
