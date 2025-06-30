use service::{ServiceDispatcher, Service, TaskId};
use crate::dummy_dispatcher::NoopDispatcher;

/// An example service for testing which always returns OK.
#[derive(Clone)]
pub struct ExampleService {}

#[derive(serde::Serialize)]
pub(crate) struct ExampleServiceRequest (pub(crate) String);
#[derive(serde::Deserialize)]

pub(crate) struct ExampleServiceResponse (pub(crate) String);

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

    fn dispatcher(&self) -> impl ServiceDispatcher<ExampleServiceRequest> {
        NoopDispatcher::new()
    }
}
