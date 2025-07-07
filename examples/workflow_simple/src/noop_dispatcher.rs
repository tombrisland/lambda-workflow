use model::Error;
use service::MessageDispatcher;

/// A noop dispatcher implementation which always succeeds, for use in testing.
pub struct NoopDispatcher {}

impl NoopDispatcher {
    pub fn new() -> Self {
        Self {}
    }
}

impl MessageDispatcher for NoopDispatcher {
    async fn send_message(&self, _: String) -> Result<(), Error> {
        Ok(())
    }
}
