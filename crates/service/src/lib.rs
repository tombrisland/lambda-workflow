pub mod service_dummy;

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

/// Name details, Request and Response types for an asynchronous service.
/// The response is returned using a callback mechanism over SQS.
/// The trait `CallableService` is implemented for any implementation of this.
pub trait ServiceDefinition<Request, Response> : Clone
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    fn name(&self) -> &'static str;

    fn call_engine(&self) -> impl CallEngine<Request>;
}

pub trait CallEngine<Request>
where
    Request: Serialize,
{
    fn call(&self, payload: ServiceRequest<Request>) -> impl Future<Output = Result<(), Error>>;
}

pub trait CallableService<Request, Response>: ServiceDefinition<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    fn call(&self, payload: ServiceRequest<Request>) -> impl Future<Output = Result<(), Error>>;
}

// Auto implement this for ease of calling service
impl<Request, Response, S> CallableService<Request, Response> for S
where
    Request: Serialize,
    Response: DeserializeOwned,
    S: ServiceDefinition<Request, Response>,
{
    async fn call(&self, payload: ServiceRequest<Request>) -> Result<(), Error> {
        self.call_engine().call(payload).await
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
