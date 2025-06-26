use async_trait::async_trait;
use model::invocation::WorkflowInvocation;
use model::task::WorkflowTask;
use ::model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Debug, Display, Formatter};

/// Store state associated with the workflow.
/// It is up to the implementation whether invocations and calls are separated.
///
/// An invocation is the original request to the workflow.
/// A call is an asynchronous request made from within the workflow.
#[async_trait]
pub trait StateStore<WorkflowRequest: Serialize + DeserializeOwned + Clone + Send>: Send + Sync {
    async fn put_invocation(
        &self,
        invocation: WorkflowInvocation<WorkflowRequest>,
    ) -> Result<(), StateError>;
    async fn get_invocation(&self, invocation_id: &str) -> Result<WorkflowRequest, StateError>;

    async fn put_task(&self, task: WorkflowTask) -> Result<(), StateError>;
    async fn get_task(
        &self,
        invocation_id: &str,
        task_id: &str,
    ) -> Result<WorkflowTask, StateError>;
}

/// Errors arising from parsing state.
#[derive(Debug)]
pub struct StateError {
    pub state_key: String,

    pub operation: StateOperation,
    pub reason: StateErrorReason,
}

#[derive(Debug)]
pub enum StateErrorReason {
    // An expected state entry was missing.
    MissingEntry,
    MissingPayload,
    // The state was not of the expected type
    BadState(String),
    // An error from the underlying state store
    BackendFailure(Error),
}

#[derive(Debug, Clone)]
pub enum StateOperation {
    GetInvocation,
    PutInvocation,
    GetTask,
    PutTask,
}

impl StateError {
    pub fn new(state_key: String, operation: StateOperation, reason: StateErrorReason) -> Self {
        StateError {
            state_key,
            operation,
            reason,
        }
    }
}

impl Display for StateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for StateError {}
