use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use aws_lambda_events::sqs::{SqsEventObj, SqsMessageObj};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub trait WorkflowId {
    fn workflow_id(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkflowEvent<T: Clone + WorkflowId> {
    Request(T),
    Update(CallResult),
}

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

impl<T: Debug + Clone + DeserializeOwned + WorkflowId> WorkflowId for WorkflowEvent<T> {
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

impl From<Error> for WorkflowError {
    fn from(value: Error) -> Self {
        // TODO properly deal with this
        WorkflowError::Error(value.to_string())
    }
}

pub type WorkflowSqsEvent<T> = SqsEventObj<WorkflowEvent<T>>;
pub type WorkflowSqsMessage<T> = SqsMessageObj<WorkflowEvent<T>>;
