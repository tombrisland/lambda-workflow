use async_trait::async_trait;
use model::Error;
use service::Dispatcher;
use std::sync::Arc;

pub struct SqsDispatcher {
    pub sqs: aws_sdk_sqs::Client,
    pub queue_url: String,
}

impl SqsDispatcher {
    pub fn new(sqs: aws_sdk_sqs::Client, queue_url: String) -> Arc<Self> {
        Arc::new(Self { sqs, queue_url })
    }
}

#[async_trait]
impl Dispatcher for SqsDispatcher {
    async fn send_message(&self, body: String) -> Result<(), Error> {
        self.sqs
            .send_message()
            .queue_url(self.queue_url.as_str())
            .message_body(body)
            .send()
            .await?;

        Ok(())
    }
}
