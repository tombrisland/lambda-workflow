use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;

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
    #[allow(async_fn_in_trait)]
    async fn call(&self, request: Request) -> Result<(), Error>;
}