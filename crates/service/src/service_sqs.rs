use crate::{CallEngine, ServiceRequest};
use model::Error;
use serde::Serialize;
use std::marker::PhantomData;
use std::rc::Rc;

pub struct SqsEngine<Request> {
    pub sqs_client: Rc<aws_sdk_sqs::Client>,
    pub queue_url: String,

    _request: PhantomData<Request>,
}

impl<'a, Request> SqsEngine<Request> {
    pub fn new(sqs_client: Rc<aws_sdk_sqs::Client>, queue_url: String) -> Self {
        Self {
            sqs_client,
            queue_url,
            _request: Default::default(),
        }
    }
}

impl<Request> CallEngine<Request> for SqsEngine<Request>
where
    Request: Serialize,
{
    async fn call(&self, body: ServiceRequest<Request>) -> Result<(), Error> {
        self.sqs_client
            .send_message()
            .queue_url(self.queue_url.as_str())
            .message_body(serde_json::to_string(&body)?)
            .send()
            .await?;

        Ok(())
    }
}
