use crate::{CallEngine, ServiceRequest};
use model::Error;
use serde::Serialize;
use std::marker::PhantomData;

/// A Dummy service which always succeeds, for use in testing.
pub struct DummyCall<Request> {
    phantom: PhantomData<Request>,
}

impl<Request> DummyCall<Request> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData::default(),
        }
    }
}

impl<Request> CallEngine<Request> for DummyCall<Request>
where
    Request: Serialize,
{
    async fn call(&self, _: ServiceRequest<Request>) -> Result<(), Error> {
        Ok(())
    }
}
