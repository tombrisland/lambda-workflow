use aws_lambda_events::sqs::SqsMessageObj;
use aws_sdk_sqs::operation::send_message::SendMessageOutput;
use aws_smithy_mocks::{Rule, mock, mock_client};
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

/// A default mock SQS client which returns an empty response
pub fn create_mock_sqs_client() -> aws_sdk_sqs::Client {
    let send_message_rule: Rule = mock!(aws_sdk_sqs::Client::send_message)
        .match_requests(|_| true)
        .sequence()
        .output(|| SendMessageOutput::builder().build())
        .repeatedly()
        .build();

    mock_client!(aws_sdk_sqs, [&send_message_rule])
}
