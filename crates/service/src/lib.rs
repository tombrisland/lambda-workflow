use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Serialize)]
pub struct ServiceRequest<Request: serde::Serialize> {
    pub invocation_id: String,
    // The task id is used as an idempotency key
    pub task_id: String,
    // TODO can we make this more generic
    pub callback_url: String,
    pub payload: Request,
}

impl<Request: serde::Serialize + TaskId> ServiceRequest<Request> {
    pub fn new(payload: Request, invocation_id: String, callback_url: String) -> Self {
        let task_id: String = payload.task_id().to_string();

        Self {
            task_id,
            invocation_id,
            callback_url,
            payload,
        }
    }
}

pub trait TaskId {
    fn task_id(&self) -> &str;
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
    fn dispatcher(&self) -> impl ServiceDispatcher<Request>;
}

pub trait ServiceDispatcher<Request>
where
    Request: Serialize,
{
    fn make_request(
        &self,
        payload: ServiceRequest<Request>,
    ) -> impl Future<Output = Result<(), Error>>;
}

/// Errors arising from parsing state.
#[derive(Debug)]
pub enum ServiceError {
    // The service returned an invalid response
    BadResponse(String),
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for ServiceError {}
