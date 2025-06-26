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





/// Use the specified `Handler` to process a batch of SQS messages.
pub async fn handle_sqs_batch<Handler, Body, HandlerFuture>(
    handler: Handler,
    event: LambdaEvent<SqsEventObj<Body>>,
) -> Result<SqsBatchResponse, Error>
where
    Handler: Fn(Body) -> HandlerFuture,
    HandlerFuture: Future<Output = Result<(), WorkflowError>>,
    Body: DeserializeOwned + Serialize + Clone,
{
    let records: Vec<SqsMessageObj<Body>> = event.payload.records;

    tracing::debug!(records = records.len(), "Received batch of SQS messages");

    // Start a task for each SQS message
    let (ids, tasks): (Vec<String>, Vec<_>) = records
        .into_iter()
        .map(|message: SqsMessageObj<Body>| {
            // We need to keep the message_id to report failures to SQS
            let message_id: String = message.message_id.unwrap_or_default();
            let body: Body = message.body;

            let message_span: Span =
                tracing::span!(tracing::Level::INFO, "SQS Handler", message_id);

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

pub fn collect_batch_failures(
    results: Zip<IntoIter<String>, IntoIter<Result<(), WorkflowError>>>,
) -> Vec<BatchItemFailure> {
    results
        .filter_map(
            // Keep message ids only where failure was not Suspended
            |(message_id, result): (String, Result<(), WorkflowError>)| match result {
                Ok(()) => None,
                Err(workflow_err) => match workflow_err {
                    WorkflowError::Suspended => None,
                    WorkflowError::Error(err) => {
                        tracing::error!("Failed to process msg {message_id} with {err}");

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

#[cfg(test)]
mod tests {
    use crate::batch_handler::handle_sqs_batch;
    use aws_lambda_events::sqs::SqsEventObj;
    use lambda_runtime::{Context, LambdaEvent};
    use model::WorkflowError;
    use model::WorkflowError::Suspended;
    use state::StateError;
    use state::StateErrorReason::MissingEntry;
    use state::StateOperation::GetTask;
    use test_utils::sqs_message_with_body;

    #[tokio::test]
    async fn test_successful_batch() {
        // NOOP handler
        let handler = async |_req: String| {
            return Result::<(), WorkflowError>::Ok(());
        };

        // Two messages with distinct values
        let sqs_event = SqsEventObj {
            records: vec![
                sqs_message_with_body("value 1".to_string()),
                sqs_message_with_body("value 2".to_string()),
            ],
        };

        let response = handle_sqs_batch(handler, LambdaEvent::new(sqs_event, Context::default()))
            .await
            .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 0));
    }

    #[tokio::test]
    async fn test_suspension_err_ignored() {
        // Handler which will throw suspension error
        let handler = async |_req: String| {
            return Result::<(), WorkflowError>::Err(Suspended);
        };

        let sqs_event = SqsEventObj {
            records: vec![sqs_message_with_body("value 1".to_string())],
        };

        let response = handle_sqs_batch(handler, LambdaEvent::new(sqs_event, Context::default()))
            .await
            .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 0));
    }

    #[tokio::test]
    async fn test_single_item_fail() {
        // Throw only on item 2
        let handler = async |req: String| {
            return if req == "value 1" {
                Result::<(), WorkflowError>::Ok(())
            } else {
                Result::<(), WorkflowError>::Err(WorkflowError::Error(
                    StateError {
                        state_key: "value 1".to_string(),
                        operation: GetTask,
                        reason: MissingEntry,
                    }
                    .into(),
                ))
            };
        };

        let sqs_event = SqsEventObj {
            records: vec![
                sqs_message_with_body("value 1".to_string()),
                sqs_message_with_body("value 2".to_string()),
            ],
        };

        let response = handle_sqs_batch(handler, LambdaEvent::new(sqs_event, Context::default()))
            .await
            .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 1));
    }

    #[tokio::test]
    #[ignore]
    /// TODO catch panics as well as errors
    /// See panic within the lambda_runtime crate
    async fn test_single_item_panic() {
        // Throw only on item 2
        let handler = async |req: String| {
            return if req == "value 1" {
                Result::<(), WorkflowError>::Ok(())
            } else {
                panic!("Batch handler should catch this")
            };
        };

        let sqs_event = SqsEventObj {
            records: vec![sqs_message_with_body("value 1".to_string())],
        };

        let response = handle_sqs_batch(handler, LambdaEvent::new(sqs_event, Context::default()))
            .await
            .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 1));
    }
}
