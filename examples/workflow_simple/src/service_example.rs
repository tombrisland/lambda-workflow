use crate::noop_dispatcher::NoopDispatcher;
use service::{Dispatcher, Service};
use std::sync::Arc;
use model::task::TaskId;

/// An example service for testing which always returns OK.
#[derive(Clone)]
pub struct ExampleService {}

#[derive(serde::Serialize)]
pub(crate) struct ExampleServiceRequest(pub(crate) String);
#[derive(serde::Deserialize)]

pub(crate) struct ExampleServiceResponse(pub(crate) String);

impl TaskId for ExampleServiceRequest {
    fn task_id(&self) -> &str {
        self.0.as_str()
    }
}

impl TaskId for ExampleServiceResponse {
    fn task_id(&self) -> &str {
        self.0.as_str()
    }
}

impl Service<ExampleServiceRequest, ExampleServiceResponse> for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        NoopDispatcher::new()
    }
}
