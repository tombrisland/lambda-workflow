use crate::service::AsyncService;
use aws_types::sdk_config::Builder;
use lambda_runtime::Error;
use serde::Serialize;

pub struct SqsService {
    queue_url: String,
}

impl SqsService {
    pub fn new(queue_url: String) -> Self {
        SqsService {
            queue_url,
        }
    }
}

impl<T> AsyncService<T> for SqsService
where
    T: Serialize
{
    async fn call(&self, request: T) -> Result<(), Error> {
        // TODO pull this out or at least make it init globally
        let builder: Builder = aws_types::SdkConfig::builder();
        let client: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(&builder.build());

        let body: String = serde_json::ser::to_string(&request)?;

        client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .send()
            .await?;

        Ok(())
    }
}
