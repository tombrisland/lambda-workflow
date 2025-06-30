use aws_sdk_sqs::Client;
use serde::{Deserialize, Serialize};
use service::{MessageDispatcher, Service, ServiceRequest, TaskId};
use service_sqs::SqsDispatcher;
use std::sync::Arc;

#[derive(Clone)]
pub struct NameService {
    sqs_client: Arc<aws_sdk_sqs::Client>,
    queue_url: String,
}

const QUEUE_URL: &'static str = "SQS_GREETER_SERVICE_QUEUE_URL";

#[derive(Serialize, Debug)]
pub(crate) struct NameRequest {
    pub(crate) first_letter: String,
}

impl TaskId for NameRequest {
    fn task_id(&self) -> &str {
        self.first_letter.as_str()
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct NameResponse {
    pub(crate) first_letter: String,
    pub(crate) name: String,
}

impl TaskId for NameResponse {
    fn task_id(&self) -> &str {
        self.first_letter.as_str()
    }
}

impl NameService {
    pub fn new(sqs_client: Arc<Client>) -> Self {
        let queue_url: String = std::env::var(QUEUE_URL)
            .expect(format!("Missing {} environment variable", QUEUE_URL).as_str());

        NameService {
            sqs_client,
            queue_url,
        }
    }
}

impl Service<NameRequest, NameResponse> for NameService {
    fn name(&self) -> &'static str {
        "NameService"
    }

    fn dispatcher(&self) -> impl MessageDispatcher<ServiceRequest<NameRequest>> {
        SqsDispatcher::new(self.sqs_client.clone(), self.queue_url.clone())
    }
}
