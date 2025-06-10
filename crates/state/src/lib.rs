use async_trait::async_trait;
use model::CallState;
use ::model::Error;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display, Formatter};

/// Store state associated with the workflow.
/// It is up to the implementation whether invocations and calls are separated.
///
/// An invocation is the original request to the workflow.
/// A call is an asynchronous request made from within the workflow.
#[async_trait]
pub trait StateStore<T: DeserializeOwned + Clone + Send> {
    async fn put_invocation(&self, invocation_id: &str, request: T) -> Result<(), StateError>;
    async fn get_invocation(&self, invocation_id: &str) -> Result<T, StateError>;

    async fn put_call(
        &self,
        invocation_id: &str,
        call_id: &str,
        state: CallState,
    ) -> Result<(), StateError>;
    async fn get_call(&self, invocation_id: &str, call_id: &str) -> Result<CallState, StateError>;
}

/// Errors arising from parsing state.
#[derive(Debug)]
pub enum StateError {
    // An expected state item was missing from the store.
    MissingState(String),
    BadState { key: String, reason: BadStateReason },
    // An error from the underlying state store
    BackendFailure(Error),
}

#[derive(Debug)]
pub enum BadStateReason {
    MissingPayload,
    BadPayload(Option<String>),
}

impl Display for StateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for StateError {}
