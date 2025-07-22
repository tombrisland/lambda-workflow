use async_trait::async_trait;
use model::task::TaskId;
use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Serialize)]
pub struct ServiceRequest<Request: serde::Serialize> {
    pub invocation_id: String,
    // The task id is used as an idempotency key
    pub task_id: String,
    // Callback to allow the service to respond
    pub callback_queue_url: String,
    pub payload: Request,
}

impl<Request: serde::Serialize + TaskId> ServiceRequest<Request> {
    pub fn new(
        invocation_id: String,
        task_id: String,
        callback_queue_url: String,
        payload: Request,
    ) -> Self {
        Self {
            invocation_id,
            task_id,
            callback_queue_url,
            payload,
        }
    }
}

/// Name details, Request and Response types for an asynchronous service.
/// The response is returned using a callback mechanism over SQS.
/// The trait `CallableService` is implemented for any implementation of this.
pub trait Service<Request, Response>: Clone
where
    Request: Serialize + TaskId,
    Response: DeserializeOwned + TaskId,
{
    fn name(&self) -> &'static str;

    /// The dispatcher implementation with which to make the request
    /// Most common is an SqsDispatcher implementation
    fn dispatcher(&self) -> Arc<dyn Dispatcher>;
}

#[async_trait]
pub trait Dispatcher {
    async fn send_message(&self, payload: String) -> Result<(), Error>;
}

/// Errors arising from parsing state.
#[derive(Debug)]
pub enum ServiceError {
    // The service request couldn't be parsed
    BadRequest(Error),
    // The service returned an invalid response
    BadResponse(String),
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for ServiceError {}
