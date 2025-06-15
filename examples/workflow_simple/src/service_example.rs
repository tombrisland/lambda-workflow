use model::Error;
use service::Service;

/// An example service for testing which always returns OK.
pub struct ExampleService {}

impl Service<String, String> for ExampleService {
    fn name(&self) -> &'static str {
        "ExampleService"
    }

    async fn call(&self, _request: String) -> Result<(), Error> {
        Ok(())
    }
}