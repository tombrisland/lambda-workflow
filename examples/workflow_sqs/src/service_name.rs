use serde::{Deserialize, Serialize};
use service::service_sqs::SqsEngine;
use service::{CallEngine, ServiceDefinition};
use std::rc::Rc;

#[derive(Clone)]
pub struct NameService {
    sqs_client: Rc<aws_sdk_sqs::Client>,
    queue_url: String,
}

const QUEUE_URL: &'static str = "SQS_GREETER_SERVICE_QUEUE_URL";

#[derive(Serialize, Debug)]
pub(crate) struct NameRequest {
    pub(crate) first_letter: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct NameResponse {
    pub(crate) name: String,
}

impl NameService {
    pub fn new(sqs_client: Rc<aws_sdk_sqs::Client>) -> Self {
        let queue_url: String = std::env::var(QUEUE_URL)
            .expect(format!("Missing {} environment variable", QUEUE_URL).as_str());

        NameService {
            sqs_client,
            queue_url,
        }
    }
}

impl ServiceDefinition<NameRequest, NameResponse> for NameService {
    fn name(&self) -> &'static str {
        "NameService"
    }

    fn call_engine(&self) -> impl CallEngine<NameRequest> {
        SqsEngine::new(self.sqs_client.clone(), self.queue_url.clone())
    }
}
