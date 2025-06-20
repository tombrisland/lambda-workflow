use service::{CallType, Service};
use service::dummy_service::DummyCall;

/// An example service for testing which always returns OK.
pub struct ExampleService {}

impl Service<String, String> for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }

    fn call_type(&self) -> impl CallType<String> {
        DummyCall::new()
    }
}
