use serde::de::StdError;
use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallResult {
    pub workflow_id: String,
    pub call_id: String,

    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub enum CallState {
    Running,
    Completed(CallResult),
}

pub trait WorkflowId {
    fn workflow_id(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkflowEvent<T: WorkflowId> {
    Request(T),
    Update(CallResult),
}

impl<T: WorkflowId> WorkflowId for WorkflowEvent<T> {
    fn workflow_id(&self) -> &str {
        match self {
            WorkflowEvent::Request(request) => request.workflow_id(),
            WorkflowEvent::Update(result) => result.workflow_id.as_str(),
        }
    }
}

#[derive(Debug)]
pub enum WorkflowError {
    Suspended,
    #[allow(dead_code)]
    Error(String),
}

impl From<Box<dyn StdError + Send + Sync>> for WorkflowError {
    fn from(value: Box<dyn StdError + Send + Sync>) -> Self {
        // TODO properly deal with this
        WorkflowError::Error(value.to_string())
    }
}
