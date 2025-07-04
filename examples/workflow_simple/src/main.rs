mod noop_dispatcher;
mod service_example;

use crate::service_example::{ExampleService, ExampleServiceRequest, ExampleServiceResponse};
use aws_config::BehaviorVersion;
use lambda_runtime::tracing;
use model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use service::WorkflowCallback;
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::context::WorkflowContext;
use workflow::runtime::{SqsBatchPublisher, WorkflowRuntime};
use workflow::workflow_fn;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestExample {
    pub(crate) id: String,
    pub(crate) item_id: String,
}

impl InvocationId for RequestExample {
    fn invocation_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseExample {
    id: String,
    item_id: String,
    payload: String,
}

impl InvocationId for ResponseExample {
    fn invocation_id(&self) -> &str {
        self.id.as_str()
    }
}

async fn workflow_example(
    ctx: WorkflowContext<RequestExample>,
) -> Result<ResponseExample, WorkflowError> {
    let request: &RequestExample = ctx.request();

    tracing::info!("Making a request in an example workflow: {:?}", request);

    let service_request: ExampleServiceRequest = ExampleServiceRequest(request.item_id.clone());

    let result: ExampleServiceResponse = ctx.call(&ExampleService {}, service_request).await?;

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result.0,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let sqs: aws_sdk_sqs::Client =
        aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

    let runtime: WorkflowRuntime<RequestExample, ResponseExample> = WorkflowRuntime::new(
        Arc::new(InMemoryStateStore::default()),
        WorkflowCallback::Noop,
        SqsBatchPublisher::new(sqs, "".to_string()),
    );

    lambda_runtime::run(workflow_fn(&runtime, workflow_example)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lambda_events::sqs::{SqsBatchResponse, SqsEventObj};
    use lambda_runtime::{Context, LambdaEvent, Service};
    use model::task::CompletedTask;
    use model::{WorkflowEvent, WorkflowSqsEvent};
    use service::WorkflowCallback;
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use test_utils::{create_mock_sqs_client, sqs_message_with_body};
    use workflow::runtime::{SqsBatchPublisher, WorkflowRuntime};
    use workflow::WorkflowLambdaEvent;

    #[tokio::test]
    async fn simple_workflow_runs() {
        let runtime: WorkflowRuntime<RequestExample, ResponseExample> = WorkflowRuntime::new(
            Arc::new(InMemoryStateStore::default()),
            WorkflowCallback::Noop,
            SqsBatchPublisher::new(create_mock_sqs_client(), "".to_string()),
        );

        let request = WorkflowEvent::Request(RequestExample {
            id: "id_1".to_string(),
            item_id: "item_1".to_string(),
        });

        let request2: WorkflowEvent<RequestExample> = WorkflowEvent::Update(CompletedTask {
            invocation_id: "id_1".to_string(),
            task_id: "item_1".to_string(),
            // Value must be valid JSON
            payload: "\"value 1\"".to_string().parse().unwrap(),
        });

        let sqs_event: WorkflowSqsEvent<RequestExample> = SqsEventObj {
            records: vec![
                sqs_message_with_body(request.clone()),
                sqs_message_with_body(request2.clone()),
                sqs_message_with_body(request.clone()),
            ],
        };
        let event: WorkflowLambdaEvent<RequestExample> =
            LambdaEvent::new(sqs_event, Context::default());

        let response: Result<SqsBatchResponse, Error> =
            workflow_fn(&runtime, workflow_example).call(event).await;

        tracing::info!("Batch handler results {:?}", response)
    }
}
