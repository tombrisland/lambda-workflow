use aws_lambda_events::sqs::SqsMessageObj;
use model::InvocationId;
use serde::{Deserialize, Serialize};

/// Create a dummy SQS message with a set body
pub fn sqs_message_with_body<T>(body: T) -> SqsMessageObj<T>
where
    T: Serialize + Clone,
{
    SqsMessageObj {
        message_id: None,
        receipt_handle: None,
        body,
        md5_of_body: None,
        md5_of_message_attributes: None,
        attributes: Default::default(),
        message_attributes: Default::default(),
        event_source_arn: None,
        event_source: None,
        aws_region: None,
    }
}

/// Request implementing InvocationId for test purposes
#[derive(Serialize, Deserialize, Clone)]
pub struct TestRequest(pub String);

impl InvocationId for TestRequest {
    fn invocation_id(&self) -> &str {
        &self.0
    }
}

impl From<String> for TestRequest {
    fn from(s: String) -> Self {
        TestRequest(s)
    }
}
