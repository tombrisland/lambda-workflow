use model::Error;
use serde::Serialize;
use service::MessageDispatcher;
use std::marker::PhantomData;

/// A noop dispatcher implementation which always succeeds, for use in testing.
pub struct NoopDispatcher<Request> {
    phantom: PhantomData<Request>,
}

impl<Request> NoopDispatcher<Request> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<Request> MessageDispatcher<Request> for NoopDispatcher<Request>
where
    Request: Serialize,
{
    async fn send_message(&self, _: Request) -> Result<(), Error> {
        Ok(())
    }
}
