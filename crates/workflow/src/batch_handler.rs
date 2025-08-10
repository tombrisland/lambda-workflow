use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEventObj, SqsMessageObj};
use lambda_runtime::tracing::instrument::Instrumented;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{tracing, Error, LambdaEvent};
use model::WorkflowError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::iter::Zip;
use std::marker::PhantomData;
use std::slice::Iter;
use std::vec::IntoIter;

#[derive(Clone)]
pub struct SqsBatchPublisher<Message: Serialize> {
    queue_url: String,
    sqs: aws_sdk_sqs::Client,
    _response: PhantomData<Message>,
}

impl<Message: Serialize> SqsBatchPublisher<Message> {
    async fn publish(&self, message: &Message) -> Result<(), WorkflowError> {
        self.sqs
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(serde_json::to_string(message).unwrap())
            .send()
            .await
            // Emit the SDK error if publishing fails
            .map_err(|err| WorkflowError::Error(err.into()))?;

        Ok(())
    }
}

impl<T: Serialize> SqsBatchPublisher<T> {
    pub fn new(sqs: aws_sdk_sqs::Client, queue_url: String) -> Self {
        Self {
            sqs,
            queue_url,
            _response: Default::default(),
        }
    }
}

/// Use the specified `Handler` to process a batch of SQS messages.
pub(crate) async fn handle_sqs_batch<Handler, HandlerFuture, Payload, Response: Serialize>(
    handler: Handler,
    event: LambdaEvent<SqsEventObj<Payload>>,
    // The responses from the handler are output to SQS
    publisher: SqsBatchPublisher<Response>,
) -> Result<SqsBatchResponse, Error>
where
    Handler: Fn(Payload) -> HandlerFuture,
    HandlerFuture: Future<Output = Result<Response, WorkflowError>>,
    Payload: DeserializeOwned + Serialize + Clone,
{
    let records: Vec<SqsMessageObj<Payload>> = event.payload.records;

    tracing::debug!(records = records.len(), "Received batch of SQS messages");

    // Start a task for each SQS message
    let (message_ids, workflow_tasks): (Vec<String>, Vec<_>) = records
        .into_iter()
        .map(|message: SqsMessageObj<Payload>| {
            // We need to keep the message_id to report failures to SQS
            let message_id: String = message.message_id.unwrap_or_default();
            let body: Payload = message.body;

            let message_span: Span = tracing::span!(tracing::Level::INFO, "SQS", message_id);

            let task: Instrumented<_> = async {
                let result: Result<Response, WorkflowError> = handler(body).await;

                // Output the workflow response on SQS
                if let Ok(response) = &result {
                    publisher.publish(response).await?;
                }

                result
            }
            .instrument(message_span);

            (message_id, task)
        })
        .unzip();

    // Process all messages concurrently
    let results: Vec<Result<Response, WorkflowError>> =
        futures::future::join_all(workflow_tasks).await;

    let batch_item_failures: Vec<BatchItemFailure> =
        collect_batch_failures(message_ids.iter().zip(results));

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}

pub fn collect_batch_failures<Response>(
    results: Zip<Iter<String>, IntoIter<Result<Response, WorkflowError>>>,
) -> Vec<BatchItemFailure> {
    // Collect the message ids of all failed items
    results
        .filter_map(
            // Keep message ids only where failure was not Suspended
            |(message_id, result): (&String, Result<Response, WorkflowError>)| match result {
                Ok(_) => None,
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
            item_identifier: id.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::batch_handler::{handle_sqs_batch, SqsBatchPublisher};
    use aws_lambda_events::sqs::SqsEventObj;
    use lambda_runtime::{Context, LambdaEvent};
    use model::WorkflowError;
    use model::WorkflowError::Suspended;
    use state::StateError;
    use state::StateErrorReason::MissingEntry;
    use state::StateOperation::GetTask;
    use test_utils::{create_mock_sqs_client, sqs_message_with_body};

    #[tokio::test]
    async fn successful_batch() {
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

        let response = handle_sqs_batch(
            handler,
            LambdaEvent::new(sqs_event, Context::default()),
            SqsBatchPublisher::new(create_mock_sqs_client(), "".to_string()),
        )
        .await
        .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 0));
    }

    #[tokio::test]
    async fn suspend_treated_as_success() {
        // Handler which will throw a suspension error
        let handler = async |_req: String| {
            return Result::<(), WorkflowError>::Err(Suspended);
        };

        let sqs_event = SqsEventObj {
            records: vec![sqs_message_with_body("value 1".to_string())],
        };

        let response = handle_sqs_batch(
            handler,
            LambdaEvent::new(sqs_event, Context::default()),
            SqsBatchPublisher::new(create_mock_sqs_client(), "".to_string()),
        )
        .await
        .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 0));
    }

    #[tokio::test]
    async fn single_item_fail() {
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

        let response = handle_sqs_batch(
            handler,
            LambdaEvent::new(sqs_event, Context::default()),
            SqsBatchPublisher::new(create_mock_sqs_client(), "".to_string()),
        )
        .await
        .unwrap();
        assert!(matches!(response.batch_item_failures.len(), 1));
    }
}
