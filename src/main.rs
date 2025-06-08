use crate::model::WorkflowLambdaEvent;
use crate::workflow_engine::WorkflowEngine;
use crate::workflow_example::{workflow_example, RequestExample};
use crate::workflow_handler::workflow_handler;
use lambda_runtime::{service_fn, tracing, Error};

mod in_memory_state;
mod model;
mod service;
pub mod sqs_service;
mod state;
mod workflow_engine;
mod workflow_example;
mod workflow_handler;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    let engine: WorkflowEngine<RequestExample> = WorkflowEngine::new();

    lambda_runtime::run(service_fn(
        async |event: WorkflowLambdaEvent<RequestExample>| {
            return workflow_handler(&engine, event, workflow_example).await;
        },
    ))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CallResult, WorkflowEvent, WorkflowSqsEvent, WorkflowSqsMessage};
    use crate::workflow_example::workflow_example;
    use aws_lambda_events::sqs::{SqsBatchResponse, SqsEventObj, SqsMessageObj};
    use lambda_runtime::{Context, LambdaEvent};

    #[tokio::test]
    async fn test_event_handler() {
        tracing::init_default_subscriber();

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

        let sqs_event: WorkflowSqsEvent<RequestExample> = SqsEventObj {
            records: vec![
                create_test_sqs_message(request.clone()),
                create_test_sqs_message(request2.clone()),
                create_test_sqs_message(request.clone()),
            ],
        };
        let event: WorkflowLambdaEvent<RequestExample> =
            LambdaEvent::new(sqs_event, Context::default());

        let response: Result<SqsBatchResponse, Error> =
            workflow_handler(&engine, event, workflow_example).await;

        tracing::info!("Batch handler results {:?}", response)
    }

    fn create_test_sqs_message(
        body: WorkflowEvent<RequestExample>,
    ) -> WorkflowSqsMessage<RequestExample> {
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
