use serde::{Deserialize, Serialize};
use model::Error;
use service::Service;

pub struct GreeterService<'a> {
    sqs_client: &'a aws_sdk_sqs::Client,
    queue_url: String,
}

impl<'a> GreeterService<'a> {
    pub fn new(sqs_client: &'a aws_sdk_sqs::Client) -> Self {
        let queue_url: String = std::env::var("SQS_GREETER_SERVICE_QUEUE_URL")
            .expect("Missing SQS_GREETER_SERVICE_QUEUE_URL environment variable");

        GreeterService {
            sqs_client,
            queue_url,
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct GreeterRequest {
    pub(crate) name: String,
}

#[derive(Deserialize, Debug)]

pub(crate) struct GreeterResponse {
    pub(crate) sentence: String,
}

impl<'a> Service<GreeterRequest, GreeterResponse> for GreeterService<'a> {
    async fn call(&self, request: GreeterRequest) -> Result<(), Error> {
        let body: String = serde_json::ser::to_string(&request)?;

        self.sqs_client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .send()
            .await?;

        Ok(())
    }
}
