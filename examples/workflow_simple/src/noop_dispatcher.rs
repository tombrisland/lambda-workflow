use async_trait::async_trait;
use model::Error;
use service::Dispatcher;
use std::sync::Arc;

/// A noop dispatcher implementation which always succeeds, for use in testing.
pub struct NoopDispatcher {}

impl NoopDispatcher {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

#[async_trait]
impl Dispatcher for NoopDispatcher {
    async fn send_message(&self, _: String) -> Result<(), Error> {
        Ok(())
    }
}
