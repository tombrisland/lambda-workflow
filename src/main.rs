use crate::model::{WorkflowError, WorkflowEvent, WorkflowId};
use crate::workflow_engine::{WorkflowContext, WorkflowEngine};
use crate::workflow_example::{workflow_example, RequestExample};
use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEventObj, SqsMessageObj};
use lambda_runtime::tracing::instrument::Instrumented;
use lambda_runtime::tracing::log::info;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{service_fn, tracing, Error, LambdaEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::future::Future;
use std::iter::Zip;
use std::vec::IntoIter;

mod in_memory_state;
mod logger;
mod model;
mod service;
pub mod sqs_service;
mod state;
mod workflow_engine;
mod workflow_example;
mod workflow_fn;

async fn batch_handler<Handler, Request, Fut>(
    handler: Handler,
    event: LambdaEvent<SqsEventObj<Request>>,
) -> Result<SqsBatchResponse, Error>
where
    Handler: Fn(Request) -> Fut,
    Fut: Future<Output = Result<(), WorkflowError>>,
    Request: DeserializeOwned + Serialize,
{
    let records: Vec<SqsMessageObj<Request>> = event.payload.records;

    tracing::debug!("Handling batch of [{}] from SQS", records.len());

    // Start a task for each SQS message
    let (ids, tasks): (Vec<String>, Vec<_>) = records
        .into_iter()
        .map(|message: SqsMessageObj<Request>| {
            // We need to keep the message_id to report failures to SQS
            let message_id: String = message.message_id.unwrap_or_default();
            let body: Request = message.body;

            let message_span: Span = tracing::span!(
                tracing::Level::DEBUG,
                "Handling message from SQS",
                message_id
            );

            // TODO catch panics as well as errors - see lambda runtime for reference
            let task: Instrumented<_> = async { handler(body).await }.instrument(message_span);

            (message_id, task)
        })
        .unzip();

    // Process all messages concurrently
    let results: Vec<Result<(), WorkflowError>> = futures::future::join_all(tasks).await;

    let batch_item_failures: Vec<BatchItemFailure> =
        collect_batch_failures(ids.into_iter().zip(results).into_iter());

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}

fn collect_batch_failures(
    results: Zip<IntoIter<String>, IntoIter<Result<(), WorkflowError>>>,
) -> Vec<BatchItemFailure> {
    results
        .filter_map(
            // Keep message ids where failure was not Suspended
            |(message_id, result): (String, Result<(), WorkflowError>)| match result {
                Ok(()) => None,
                Err(workflow_err) => match workflow_err {
                    WorkflowError::Suspended => None,
                    WorkflowError::Error(err) => {
                        tracing::error!("Failed to process msg {message_id}, {err}");

                        Some(message_id)
                    }
                },
            },
        )
        .map(|id| BatchItemFailure {
            item_identifier: id,
        })
        .collect()
}

pub(crate) async fn workflow_wrapper<Fut, Request, Response>(
    engine: &WorkflowEngine<Request>,
    event: WorkflowEvent<Request>,
    workflow: fn(WorkflowContext<Request>) -> Fut,
) -> Result<(), WorkflowError>
where
    Request: DeserializeOwned + Serialize + Clone + WorkflowId + Debug + 'static,
    Response: Serialize,
    Fut: Future<Output = Result<Response, WorkflowError>>,
{
    let ctx: WorkflowContext<Request> = engine.accept(event)?;

    let _result = workflow(ctx).await;

    info!("Finishing workflow event handler");

    Ok(())
}

type WorkflowSqsEvent<T> = SqsEventObj<WorkflowEvent<T>>;
type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let engine: WorkflowEngine<RequestExample> = WorkflowEngine::new();
    let engine_ref: &WorkflowEngine<RequestExample> = &engine;

    lambda_runtime::run(service_fn(
        move |event: WorkflowLambdaEvent<RequestExample>| async move {
            return batch_handler(
                |request| workflow_wrapper(engine_ref, request, workflow_example),
                event,
            )
            .await;
        },
    ))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::StdoutLogger;
    use crate::model::CallResult;
    use crate::workflow_example::workflow_example;
    use lambda_runtime::tracing::log;
    use lambda_runtime::tracing::log::LevelFilter;
    use lambda_runtime::{Context, LambdaEvent};

    static LOGGER: StdoutLogger = StdoutLogger;

    #[tokio::test]
    async fn test_event_handler() {
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();

        let engine: WorkflowEngine<RequestExample> = WorkflowEngine::new();

        let request = WorkflowEvent::Request(RequestExample {
            id: "id_1".to_string(),
            item_id: "item_1".to_string(),
        });

        let request2: WorkflowEvent<RequestExample> = WorkflowEvent::Update(CallResult {
            workflow_id: "id_1".to_string(),
            call_id: "item_1".to_string(),
            value: "value 1".to_string(),
        });

        let sqs_event: SqsEventObj<WorkflowEvent<RequestExample>> = SqsEventObj {
            records: vec![
                create_test_sqs_message(request.clone()),
                create_test_sqs_message(request2.clone()),
                create_test_sqs_message(request.clone()),
            ],
        };
        let event: LambdaEvent<SqsEventObj<WorkflowEvent<RequestExample>>> =
            LambdaEvent::new(sqs_event, Context::default());

        let response: SqsBatchResponse = batch_handler(
            |request| workflow_wrapper(&engine, request, workflow_example),
            event,
        )
            .await
            .unwrap();
        
        info!("Batch handler results {:?}", response)
    }

    fn create_test_sqs_message(
        body: WorkflowEvent<RequestExample>,
    ) -> SqsMessageObj<WorkflowEvent<RequestExample>> {
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
}
