pub mod dummy_service;
pub mod sqs_service;

use aws_sdk_sqs::error::BoxError;
use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Serialize)]
pub struct ServiceRequest<Request: serde::Serialize> {
    // The task id is used as an idempotency key
    pub task_id: String,
    pub payload: Request,
}

/// A service which doesn't expect a response immediately.
/// The response is returned using a callback mechanism over SQS.
/// The trait `CallableService` is implemented for any implementation of this.
pub trait Service<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    fn name(&self) -> &'static str;

    fn call_type(&self) -> impl CallType<Request>;
}

pub trait CallType<Request>
where
    Request: Serialize,
{
    fn call(&self, payload: ServiceRequest<Request>) -> impl Future<Output = Result<(), Error>>;
}

pub trait CallableService<Request, Response>: Service<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    fn call(&self, payload: ServiceRequest<Request>) -> impl Future<Output = Result<(), BoxError>>;
}

// Auto implement this for ease of calling service
impl<Request, Response, S> CallableService<Request, Response> for S
where
    Request: Serialize,
    Response: DeserializeOwned,
    S: Service<Request, Response>,
{
    async fn call(&self, payload: ServiceRequest<Request>) -> Result<(), BoxError> {
        self.call_type().call(payload).await
    }
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
