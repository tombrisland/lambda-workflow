use service::{CallEngine, ServiceDefinition, TaskId};
use service::service_dummy::DummyCall;

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

impl ServiceDefinition<ExampleServiceRequest, ExampleServiceResponse> for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }

    fn call_engine(&self) -> impl CallEngine<ExampleServiceRequest> {
        DummyCall::new()
    }
}
