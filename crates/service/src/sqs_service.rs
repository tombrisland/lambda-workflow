use crate::{CallType, ServiceRequest};
use model::Error;
use serde::Serialize;
use std::marker::PhantomData;

pub struct SqsCall<'a, Request> {
    pub sqs_client: &'a aws_sdk_sqs::Client,
    pub queue_url: &'a str,

    request_type: PhantomData<Request>,
}

impl<'a, Request> SqsCall<'a, Request> {
    pub fn new(client: &'a aws_sdk_sqs::Client, queue_url: &'a str) -> Self {
        Self {
            sqs_client: client,
            queue_url,
            request_type: Default::default(),
        }
    }
}

impl<'a, Request> CallType<Request> for SqsCall<'a, Request>
where
    Request: Serialize,
{
    async fn call(&self, body: ServiceRequest<Request>) -> Result<(), Error> {
        self.sqs_client
            .send_message()
            .queue_url(self.queue_url)
            .message_body(serde_json::to_string(&body)?)
            .send()
            .await?;

        Ok(())
    }
}
