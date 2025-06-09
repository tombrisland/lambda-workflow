use serde::{Deserialize, Serialize};
use model::Error;
use service::Service;

pub struct NameService<'a> {
    sqs_client: &'a aws_sdk_sqs::Client,
    queue_url: String,
}

impl<'a> NameService<'a> {
    pub fn instance(sqs_client: &'a aws_sdk_sqs::Client) -> Self {
        let queue_url: String = std::env::var("SQS_GREETER_SERVICE_QUEUE_URL")
            .expect("Missing SQS_GREETER_SERVICE_QUEUE_URL environment variable");

        NameService {
            sqs_client,
            queue_url,
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct NameRequest {
    pub(crate) first_letter: String,
}

#[derive(Deserialize, Debug)]

pub(crate) struct NameResponse {
    pub(crate) name: String,
}

impl<'a> Service<NameRequest, NameResponse> for NameService<'a> {
    async fn call(&self, request: NameRequest) -> Result<(), Error> {
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
