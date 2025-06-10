use aws_lambda_events::sqs::{SqsEventObj, SqsMessageObj};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// This id is used for tracing and storage.
pub trait InvocationId {
    fn invocation_id(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkflowEvent<T: Clone + InvocationId> {
    Request(T),
    Update(CallResult),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallResult {
    pub workflow_id: String,
    pub call_id: String,

    // Any JSON value is acceptable
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub enum CallState {
    Running,
    Completed(CallResult),
}

impl<T: Debug + Clone + DeserializeOwned + InvocationId> InvocationId for WorkflowEvent<T> {
    fn invocation_id(&self) -> &str {
        match self {
            WorkflowEvent::Request(request) => request.invocation_id(),
            WorkflowEvent::Update(result) => result.workflow_id.as_str(),
        }
    }
}

#[derive(Debug)]
pub enum WorkflowError {
    Suspended,
    #[allow(dead_code)]
    Error(Error),
}

impl Display for WorkflowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl From<Error> for WorkflowError {
    fn from(value: Error) -> Self {
        WorkflowError::Error(value)
    }
}

pub type WorkflowSqsEvent<T> = SqsEventObj<WorkflowEvent<T>>;
pub type WorkflowSqsMessage<T> = SqsMessageObj<WorkflowEvent<T>>;
