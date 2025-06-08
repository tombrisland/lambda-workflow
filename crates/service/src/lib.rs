use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct ServiceRequest<Request: serde::Serialize> {
    // The call id is used as an idempotency key
    pub call_id: String,
    pub inner: Request,
}

/// A service which doesn't expect a response immediately.
/// The response is returned using a callback mechanism over SQS.
pub trait Service<Request, Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    #[allow(async_fn_in_trait)]
    async fn call(&self, request: Request) -> Result<(), Error>;
}

/// An example service for testing which always returns OK.
pub struct ExampleService {}

impl Service<String, String> for ExampleService {
    async fn call(&self, _request: String) -> Result<(), Error> {
        Ok(())
    }
}

pub struct ExampleSqsService<'a> {
    sqs_client: &'a aws_sdk_sqs::Client,
    queue_url: &'a str,
}

impl<'a> ExampleSqsService<'a> {
    pub fn _new(sqs_client: &'a aws_sdk_sqs::Client) -> Self {
        ExampleSqsService {
            sqs_client,
            queue_url: "https://localhost/queue",
        }
    }
}

impl<'a, Request: serde::Serialize, Response: DeserializeOwned> Service<Request, Response>
    for ExampleSqsService<'a>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    async fn call(&self, request: Request) -> Result<(), Error> {
        let body: String = serde_json::ser::to_string(&request)?;

        self.sqs_client
            .send_message()
            .queue_url(self.queue_url)
            .message_body(body)
            .send()
            .await?;

        Ok(())
    }
}
