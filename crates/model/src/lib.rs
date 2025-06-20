pub mod task;
pub mod invocation;

use aws_lambda_events::sqs::{SqsEventObj, SqsMessageObj};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
pub use crate::invocation::InvocationId;
use crate::task::CompletedTask;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WorkflowEvent<T: Clone + InvocationId> {
    Request(T),
    
    Update(CompletedTask),
}

impl<T: Debug + Clone + DeserializeOwned + InvocationId> InvocationId for WorkflowEvent<T> {
    fn invocation_id(&self) -> &str {
        match self {
            WorkflowEvent::Request(inner) => inner.invocation_id(),
            WorkflowEvent::Update(inner) => &inner.invocation_id,
        }
    }
}

#[derive(Debug)]
pub enum WorkflowError {
    Suspended,
    #[allow(dead_code)]
    Error(Error),
}


impl From<Error> for WorkflowError {
    fn from(value: Error) -> Self {
        WorkflowError::Error(value)
    }
}

pub type WorkflowSqsEvent<T> = SqsEventObj<WorkflowEvent<T>>;
pub type WorkflowSqsMessage<T> = SqsMessageObj<WorkflowEvent<T>>;
