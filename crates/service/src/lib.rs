mod service_sqs;

use serde::Serialize;
use model::Error;

/// A service which doesn't expect a response immediately.
/// The response is returned using a callback mechanism over SQS.
pub trait AsyncService<Request>
where
    Request: ?Sized + Serialize,
{
    #[allow(async_fn_in_trait)]
    async fn call(&self, request: Request) -> Result<(), Error>;
}

pub struct DummyService {}

impl<Request> AsyncService<Request> for DummyService
where
    Request: Serialize,
{
    async fn call(&self, _request: Request) -> Result<(), Error> {
        Ok(())
    }
}
