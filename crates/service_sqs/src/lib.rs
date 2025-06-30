use model::Error;
use serde::Serialize;
use service::MessageDispatcher;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct SqsDispatcher<Request> {
    pub sqs_client: Arc<aws_sdk_sqs::Client>,
    pub queue_url: String,

    _request: PhantomData<Request>,
}

impl<'a, Request> SqsDispatcher<Request> {
    pub fn new(sqs_client: Arc<aws_sdk_sqs::Client>, queue_url: String) -> Self {
        Self {
            sqs_client,
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
        self.sqs_client
            .send_message()
            .queue_url(self.queue_url.as_str())
            .message_body(serde_json::to_string(&body)?)
            .send()
            .await?;

        Ok(())
    }
}
