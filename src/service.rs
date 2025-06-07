use lambda_runtime::Error;
use serde::Serialize;

/// A service which doesn't expect a response immediately.
/// The response is returned using a callback mechanism over SQS.
pub(crate) trait AsyncService<Request>
where
    Request: ?Sized + Serialize,
{
    async fn call(&self, request: Request) -> Result<(), Error>;
}

pub struct DummyService {}

impl<T> AsyncService<T> for DummyService
where
    T: Serialize,
{
    async fn call(&self, _request: T) -> Result<(), Error> {
        Ok(())
    }
}
