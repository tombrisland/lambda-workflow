mod service_example;

use crate::service_example::{ExampleService, ExampleServiceRequest, ExampleServiceResponse};
use aws_config::BehaviorVersion;
use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEventObj, SqsMessageObj};
use lambda_runtime::tracing::instrument::Instrumented;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{tracing, LambdaEvent};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent};
use serde::{Deserialize, Serialize};
use state_in_memory::InMemoryStateStore;
use std::pin::Pin;
use std::sync::Arc;
use workflow::batch_handler::{handle_sqs_batch, collect_batch_failures};
use workflow::context::WorkflowContext;
use workflow::runtime::WorkflowRuntime;
use workflow::{WorkflowHandler, WorkflowLambdaEvent};

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

    tracing::info!("Making request in workflow example: {:?}", request);

    let service_request: ExampleServiceRequest = ExampleServiceRequest(request.item_id.clone());

    let result: ExampleServiceResponse = ctx.call(&ExampleService {}, service_request).await?;

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result.0,
    })
}

fn workflow_fn<WorkflowFuture, LambdaFuture>(
    runtime: WorkflowRuntime<RequestExample>,
    func: fn(WorkflowContext<RequestExample>) -> WorkflowFuture,
) -> impl Fn(
    LambdaEvent<SqsEventObj<WorkflowEvent<RequestExample>>>,
) -> Pin<Box<dyn Future<Output = Result<SqsBatchResponse, Error>>>>
where
    WorkflowFuture: Future<Output = Result<ResponseExample, WorkflowError>>,
    LambdaFuture: Future<Output = Result<SqsBatchResponse, Error>>,
{
    let handler = WorkflowHandler {
        runtime,
        workflow: func,
    };

    move |event: LambdaEvent<SqsEventObj<WorkflowEvent<RequestExample>>>| {
        let records: Vec<SqsMessageObj<WorkflowEvent<RequestExample>>> = event.payload.records;

        tracing::debug!(records = records.len(), "Received batch of SQS messages");

        // Start a task for each SQS message
        let (ids, tasks): (Vec<String>, Vec<WorkflowFuture>) = records
            .into_iter()
            .map(|message: SqsMessageObj<WorkflowEvent<RequestExample>>| {
                // We need to keep the message_id to report failures to SQS
                let message_id: String = message.message_id.unwrap_or_default();
                let body: WorkflowEvent<RequestExample> = message.body;

                let message_span: Span =
                    tracing::span!(tracing::Level::INFO, "SQS Handler", message_id);

                let task: Instrumented<WorkflowFuture> =
                    handler.invoke(body).instrument(message_span);

                (message_id, task)
            })
            .unzip();

        Box::pin(async move {
            // Process all messages concurrently
            let results: Vec<Result<ResponseExample, WorkflowError>> =
                futures::future::join_all(tasks).await;

            let batch_item_failures: Vec<BatchItemFailure> =
                collect_batch_failures(ids.into_iter().zip(results).into_iter());

            Ok(SqsBatchResponse {
                batch_item_failures,
            })
        })
    }
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

    lambda_runtime::run(workflow_fn(runtime, move |event| {
        workflow_example(event, example)
    }))
    .await
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
    use workflow::runtime::WorkflowRuntime;
    use workflow::WorkflowLambdaEvent;

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
