use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Display, Formatter};

pub struct ServiceRequest<Request: serde::Serialize> {
    // The call id is used as an idempotency key
    pub call_id: String,
    pub inner: Request,
}

/// A service which doesn't expect a response immediately.
/// The response is returned using a callback mechanism over SQS.
pub trait Service<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    fn name(&self) -> &'static str;
    fn call(&self, request: Request) -> impl Future<Output = Result<(), Error>>;
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
