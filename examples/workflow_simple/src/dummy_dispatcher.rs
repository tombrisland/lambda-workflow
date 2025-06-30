use model::Error;
use serde::Serialize;
use std::marker::PhantomData;
use service::{ServiceDispatcher, ServiceRequest};

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

impl<Request> ServiceDispatcher<Request> for NoopDispatcher<Request>
where
    Request: Serialize,
{
    async fn make_request(&self, _: ServiceRequest<Request>) -> Result<(), Error> {
        Ok(())
    }
}