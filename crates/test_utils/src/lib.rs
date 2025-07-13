use aws_lambda_events::sqs::SqsMessageObj;
use aws_sdk_sqs::operation::send_message::SendMessageOutput;
use aws_smithy_mocks::{mock, mock_client, Rule};
use model::env::{WORKFLOW_INPUT_QUEUE_URL, WORKFLOW_OUTPUT_QUEUE_URL};
use model::task::TaskId;
use model::InvocationId;
use serde::{Deserialize, Serialize};
use std::env;

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

/// Wrapped string implementing types for request and payload responses.
#[derive(Serialize, Deserialize, Clone)]
pub struct TestStr(pub String);

impl InvocationId for TestStr {
    fn invocation_id(&self) -> &str {
        &self.0
    }
}

impl TaskId for TestStr {
    fn task_id(&self) -> &str {
        &self.0
    }
}

impl From<String> for TestStr {
    fn from(s: String) -> Self {
        TestStr(s)
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

/// Test queue values
pub const TEST_INPUT_QUEUE: &str = "input_queue";
pub const TEST_OUTPUT_QUEUE: &str = "output_queue";

/// Setup default environment variables used in testing
pub fn setup_default_env() {
    unsafe {
        env::set_var(WORKFLOW_INPUT_QUEUE_URL, TEST_INPUT_QUEUE);
        env::set_var(WORKFLOW_OUTPUT_QUEUE_URL, TEST_OUTPUT_QUEUE);
    }
}
