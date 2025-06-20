use serde::{Deserialize, Serialize};
use service::sqs_service::SqsCall;
use service::{CallType, Service};

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
    fn name(&self) -> &'static str {
        "NameService"
    }

    fn call_type(&self) -> impl CallType<NameRequest> {
        SqsCall::new(self.sqs_client, &*self.queue_url)
    }
}
