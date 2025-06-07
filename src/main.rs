use crate::model::{WorkflowError, WorkflowEvent, WorkflowId};
use crate::service::DummyService;
use crate::workflow_engine::{WorkflowContext, WorkflowEngine};
use aws_lambda_events::sqs::SqsEvent;
use lambda_runtime::tracing::log::info;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use serde_derive::{Deserialize, Serialize};

mod in_memory_state;
mod logger;
mod model;
mod service;
pub mod sqs_service;
mod state;
mod workflow_engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestExample {
    id: String,
    item_id: String,
}

impl WorkflowId for RequestExample {
    fn workflow_id(&self) -> &str {
        &self.id
    }
}

async fn workflow_example(ctx: WorkflowContext<RequestExample>) -> Result<(), WorkflowError> {
    let request: &RequestExample = ctx.get_request();
    info!("Handling request Example");

    info!("Calling external service");

    let result = ctx.call(DummyService {}, request.item_id.as_str()).await?;

    info!("Got response from service {}", result);

    Ok(())
}

pub(crate) async fn function_handler(
    engine: &WorkflowEngine<RequestExample>,
    event: LambdaEvent<SqsEvent>,
) -> Result<(), Error> {
    info!("Starting workflow event handler");

    // Iterate the events from the SQS queue
    for sqs_message in event.payload.records.iter() {
        let body: String = sqs_message.body.clone().unwrap();
        let workflow_event: WorkflowEvent<RequestExample> = serde_json::from_str(&body).unwrap();

        info!(
            "Handling {:?} event for workflow_id {}",
            workflow_event,
            workflow_event.workflow_id()
        );

        let ctx: WorkflowContext<RequestExample> = engine.accept(workflow_event)?;

        let _result = workflow_example(ctx).await;
    }

    info!("Finishing workflow event handler");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let engine: WorkflowEngine<RequestExample> = WorkflowEngine::new();
    let engine_ref: &WorkflowEngine<RequestExample> = &engine;

    run(service_fn(move |event: LambdaEvent<SqsEvent>| async move {
        return function_handler(engine_ref, event).await;
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::StdoutLogger;
    use crate::model::CallResult;
    use aws_lambda_events::sqs::SqsMessage;
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

        let sqs_event: SqsEvent = SqsEvent {
            records: vec![
                create_test_sqs_message(serde_json::to_string(&request).unwrap()),
                create_test_sqs_message(serde_json::to_string(&request2).unwrap()),
                create_test_sqs_message(serde_json::to_string(&request).unwrap()),
            ],
        };
        let event: LambdaEvent<SqsEvent> = LambdaEvent::new(sqs_event, Context::default());

        let response = function_handler(&engine, event).await.unwrap();
        assert_eq!((), response);
    }

    fn create_test_sqs_message(body: String) -> SqsMessage {
        SqsMessage {
            message_id: None,
            receipt_handle: None,
            body: Some(body),
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
