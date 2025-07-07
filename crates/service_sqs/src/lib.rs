use model::Error;
use service::MessageDispatcher;

pub struct SqsDispatcher {
    pub sqs: aws_sdk_sqs::Client,
    pub queue_url: String,
}

impl SqsDispatcher {
    pub fn new(sqs: aws_sdk_sqs::Client, queue_url: String) -> Self {
        Self { sqs, queue_url }
    }
}

impl MessageDispatcher for SqsDispatcher {
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
