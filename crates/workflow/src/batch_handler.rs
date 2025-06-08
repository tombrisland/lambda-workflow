use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEventObj, SqsMessageObj};
use lambda_runtime::tracing::instrument::Instrumented;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{tracing, Error, LambdaEvent};
use model::WorkflowError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::iter::Zip;
use std::vec::IntoIter;

pub(crate) async fn batch_handler<Handler, Body, Fut>(
    handler: Handler,
    event: LambdaEvent<SqsEventObj<Body>>,
) -> Result<SqsBatchResponse, Error>
where
    Handler: Fn(Body) -> Fut,
    Fut: Future<Output = Result<(), WorkflowError>>,
    Body: DeserializeOwned + Serialize + Clone,
{
    let records: Vec<SqsMessageObj<Body>> = event.payload.records;

    tracing::info!("Handling batch of [{}] from SQS", records.len());

    // Start a task for each SQS message
    let (ids, tasks): (Vec<String>, Vec<_>) = records
        .into_iter()
        .map(|message: SqsMessageObj<Body>| {
            // We need to keep the message_id to report failures to SQS
            let message_id: String = message.message_id.unwrap_or_default();
            let body: Body = message.body;

            let message_span: Span =
                tracing::span!(tracing::Level::INFO, "SQS Handler", message_id);

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
