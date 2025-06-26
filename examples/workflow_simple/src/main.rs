mod service_example;

use crate::service_example::{ExampleService, ExampleServiceRequest, ExampleServiceResponse};
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, tracing};
use model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::context::WorkflowContext;
use workflow::runtime::WorkflowRuntime;
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

async fn workflow_example(
    ctx: WorkflowContext<RequestExample>,
    example: String,
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

    let sqs_client: aws_sdk_sqs::Client =
        aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

    let runtime: WorkflowRuntime<RequestExample> = WorkflowRuntime::new(
        Arc::new(InMemoryStateStore::default()),
        Arc::new(sqs_client),
    );

    let example: String = "".to_string();

    // TODO consider implementing service for response from workflow_fn
    // Could pass in owned values, create your grouped value and return it within the future to satisfy ownership
    let x = move |event| {
        workflow_example(event, example)
    };

    lambda_runtime::run(service_fn(workflow_fn(&runtime, &x))).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lambda_events::sqs::SqsEventObj;
    use lambda_runtime::{Context, LambdaEvent};
    use model::task::CompletedTask;
    use model::{WorkflowEvent, WorkflowSqsEvent};
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use test_utils::sqs_message_with_body;
    use workflow::WorkflowLambdaEvent;
    use workflow::runtime::WorkflowRuntime;

    #[tokio::test]
    async fn test_simple_workflow_runs() {
        let sqs_client: aws_sdk_sqs::Client =
            aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

        let engine: WorkflowRuntime<RequestExample> = WorkflowRuntime::new(
            Arc::new(InMemoryStateStore::default()),
            Arc::new(sqs_client),
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

        // let response: Result<SqsBatchResponse, Error> =
        //     workflow_fn(&engine, event, service_fn(workflow_example)).await;

        // tracing::info!("Batch handler results {:?}", response)
    }
}
