mod service_example;

use std::rc::Rc;
use crate::service_example::ExampleService;
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, tracing};
use model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use service::ServiceRequest;
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::runtime::{WorkflowContext, WorkflowRuntime};
use workflow::{workflow_fn, WorkflowLambdaEvent};

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
) -> Result<ResponseExample, WorkflowError> {
    let request: &RequestExample = ctx.request();

    tracing::info!("Making request in workflow example: {:?}", request);

    let service_request = ServiceRequest {
        task_id: request.item_id.clone(),
        payload: request.item_id.clone(),
    };

    let result: String = ctx.call(&ExampleService {}, service_request).await?;

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let sqs_client: aws_sdk_sqs::Client =
        aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

    let engine: WorkflowRuntime<RequestExample> =
        WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), Rc::new(sqs_client));

    lambda_runtime::run(service_fn(
        async |event: WorkflowLambdaEvent<RequestExample>| {
            return workflow_fn(&engine, event, service_fn(workflow_example)).await;
        },
    ))
    .await
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use super::*;
    use aws_lambda_events::sqs::{SqsBatchResponse, SqsEventObj};
    use lambda_runtime::{Context, LambdaEvent};
    use model::{Error, WorkflowEvent, WorkflowSqsEvent};
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use model::task::CompletedTask;
    use test_utils::sqs_message_with_body;
    use workflow::runtime::WorkflowRuntime;
    use workflow::{workflow_fn, WorkflowLambdaEvent};

    #[tokio::test]
    async fn test_simple_workflow_runs() {
        tracing::init_default_subscriber();

        let sqs_client: aws_sdk_sqs::Client =
            aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

        let engine: WorkflowRuntime<RequestExample> =
            WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), Rc::new(sqs_client));

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
            workflow_fn(&engine, event, service_fn(workflow_example)).await;

        tracing::info!("Batch handler results {:?}", response)
    }
}
