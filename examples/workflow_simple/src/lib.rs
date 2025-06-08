use lambda_runtime::{service_fn, tracing};
use model::{Error, WorkflowError, WorkflowId};
use serde::{Deserialize, Serialize};
use service::DummyService;
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::engine::{WorkflowContext, WorkflowEngine};
use workflow::{workflow_handler, WorkflowLambdaEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestExample {
    pub(crate) id: String,
    pub(crate) item_id: String,
}

impl WorkflowId for RequestExample {
    fn workflow_id(&self) -> &str {
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
    let request: &RequestExample = ctx.get_request();

    let result: String = ctx.call(DummyService {}, request.item_id.as_str()).await?;

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let engine: WorkflowEngine<RequestExample> =
        WorkflowEngine::new(Arc::new(InMemoryStateStore::default()));

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
    use aws_lambda_events::sqs::{SqsBatchResponse, SqsEventObj, SqsMessageObj};
    use lambda_runtime::{tracing, Context, LambdaEvent};
    use model::{CallResult, Error, WorkflowEvent, WorkflowSqsEvent, WorkflowSqsMessage};
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use workflow::engine::WorkflowEngine;
    use workflow::{workflow_handler, WorkflowLambdaEvent};

    #[tokio::test]
    async fn test_event_handler() {
        tracing::init_default_subscriber();

        let engine: WorkflowEngine<RequestExample> =
            WorkflowEngine::new(Arc::new(InMemoryStateStore::default()));

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
