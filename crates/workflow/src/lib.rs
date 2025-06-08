use std::fmt::Debug;
use aws_lambda_events::sqs::SqsBatchResponse;
use lambda_runtime::{tracing, LambdaEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use model::{Error, WorkflowError, WorkflowEvent, WorkflowId, WorkflowSqsEvent};
use crate::batch_handler::batch_handler;
use crate::engine::{WorkflowContext, WorkflowEngine};

pub mod engine;
mod batch_handler;

/// Creates a handler function for a workflow designed for use with `lambda_runtime::run()`
///
/// Expects the function to receive an `SqsEvent` and returns an `SqsBatchResponse`.
/// Therefore, the function *must* have `ReportBatchItemFailures` set to true. 
///
/// # Example
/// ```no_run
/// use lambda_runtime::{Error, service_fn, LambdaEvent};
/// use lambda_workflow::{WorkflowEngine, WorkflowContext, WorkflowLambdaEvent, WorkflowError};
/// use serde_json::Value;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let engine: WorkflowEngine<Value> = WorkflowEngine::new();
///
///     let service_func = service_fn(
///        async |event: WorkflowLambdaEvent<Value>| {
///            return lambda_workflow::workflow_handler(&engine, event, workflow_example).await;
///        }
///     ));
///     lambda_runtime::run(service_func).await?;
///
///     Ok(())
/// }
///
/// async fn workflow(ctx: WorkflowContext<Value>) -> Result<Value, WorkflowError> {
///     Ok(())
/// }
/// ```
pub async fn workflow_handler<Fut, Request, Response>(
    engine: &WorkflowEngine<Request>,
    event: WorkflowLambdaEvent<Request>,
    workflow: fn(WorkflowContext<Request>) -> Fut,
) -> Result<SqsBatchResponse, Error>
where
    Request: DeserializeOwned + Serialize + Clone + WorkflowId + Debug + 'static,
    Response: Serialize + Debug,
    Fut: Future<Output = Result<Response, WorkflowError>>,
{
    batch_handler(async |request: WorkflowEvent<Request>| {
        let ctx: WorkflowContext<Request> = engine.accept(request)?;

        let response: Response = workflow(ctx).await?;

        tracing::info!("Response from handler {:?}", response);

        Ok(())
    }, event).await
}

pub type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;