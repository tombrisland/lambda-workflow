use model::Error;
use serde::Serialize;
use service::MessageDispatcher;
use std::marker::PhantomData;

pub struct SqsDispatcher<Request> {
    pub sqs: aws_sdk_sqs::Client,
    pub queue_url: String,

    _request: PhantomData<Request>,
}

impl<'a, Request> SqsDispatcher<Request> {
    pub fn new(sqs: aws_sdk_sqs::Client, queue_url: String) -> Self {
        Self {
            sqs,
            queue_url,
            _request: Default::default(),
        }
    }
}

impl<Request> MessageDispatcher<Request> for SqsDispatcher<Request>
where
    Request: Serialize,
{
    async fn send_message(&self, body: Request) -> Result<(), Error> {
        self.sqs
            .send_message()
            .queue_url(self.queue_url.as_str())
            .message_body(serde_json::to_string(&body)?)
            .send()
            .await?;

        Ok(())
    }
}
