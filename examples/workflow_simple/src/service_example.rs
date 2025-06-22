use service::{CallEngine, ServiceDefinition};
use service::service_dummy::DummyCall;

/// An example service for testing which always returns OK.
#[derive(Clone)]
pub struct ExampleService {}

impl ServiceDefinition<String, String> for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }

    fn call_engine(&self) -> impl CallEngine<String> {
        DummyCall::new()
    }
}
