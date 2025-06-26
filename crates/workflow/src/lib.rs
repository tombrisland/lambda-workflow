use crate::batch_handler::handle_sqs_batch;
use crate::context::WorkflowContext;
use crate::runtime::WorkflowRuntime;
use aws_lambda_events::sqs::SqsBatchResponse;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{LambdaEvent, tracing};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent, WorkflowSqsEvent};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

mod batch_handler;
pub mod context;
pub mod runtime;

/// Creates a handler function for a workflow designed for use with `lambda_runtime::run()`
///
/// Expects the function to receive an `SqsEvent` and returns an `SqsBatchResponse`.
/// Therefore, the function *must* have `ReportBatchItemFailures` set to true.
///
/// ```no_compile
/// use lambda_runtime::{service_fn, LambdaEvent};
/// use workflow::runtime::{WorkflowRuntime, WorkflowContext};
/// use workflow::{WorkflowLambdaEvent, workflow_fn};
/// use model::{InvocationId, Error};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Serialize, Deserialize, Debug)]
/// struct ExampleRequest {
///     id: String,
/// }
///
/// impl InvocationId for ExampleRequest {
///     fn invocation_id(&self) -> &str {
///        &self.id
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let engine: WorkflowRuntime<ExampleRequest> = WorkflowRuntime::new(Arc::new(()), ());
///
///     let service_func = service_fn(
///        async |event: WorkflowLambdaEvent<ExampleRequest>| {
///            return workflow_fn(&engine, event, workflow_example).await;
///        }
///     ));
///     lambda_runtime::run(service_func).await?;
///
///     Ok(())
/// }
///
/// async fn workflow_example(ctx: WorkflowContext<Value>) -> Result<Value, WorkflowError> {
///     Ok(())
/// }
/// ```
pub fn workflow_fn<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, LambdaFuture>(
    runtime: &'a WorkflowRuntime<WorkflowRequest>,
    workflow: &'a fn(WorkflowContext<WorkflowRequest>) -> WorkflowFuture,
) -> impl Fn(
    WorkflowLambdaEvent<WorkflowRequest>,
) -> Pin<Box<dyn Future<Output = Result<SqsBatchResponse, Error>> + 'a>>
where
    WorkflowRequest: DeserializeOwned + Clone + InvocationId + Serialize + Debug + Send,
    WorkflowResponse: DeserializeOwned + Clone + InvocationId + Serialize + Debug + 'a,
    WorkflowFuture: Future<Output = Result<WorkflowResponse, WorkflowError>>,
    LambdaFuture: Future<Output = Result<SqsBatchResponse, Error>>,
{
    // Handler for each message
    let handler = move |request: WorkflowEvent<WorkflowRequest>| async move {
        let invocation_id: String = request.invocation_id().to_string();
        let ctx: WorkflowContext<WorkflowRequest> = runtime.accept(request).await?;

        let workflow_span: Span = tracing::span!(tracing::Level::INFO, "Workflow", invocation_id);
        let response: WorkflowResponse = workflow(ctx).instrument(workflow_span).await?;

        // For now just log the response
        tracing::info!("Completed workflow {:?}", response);
        Ok(response)
    };

    // Handler for the outer Lambda event
    move |event: WorkflowLambdaEvent<WorkflowRequest>| Box::pin(handle_sqs_batch(handler, event))
}

pub type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;
