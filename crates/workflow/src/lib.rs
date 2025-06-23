use crate::batch_handler::batch_handler;
use crate::runtime::{WorkflowRuntime};
use aws_lambda_events::sqs::SqsBatchResponse;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{tracing, LambdaEvent, Service};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent, WorkflowSqsEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use crate::context::WorkflowContext;

mod batch_handler;
pub mod runtime;
pub mod context;

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
pub async fn workflow_fn<Request, Response, S>(
    engine: &WorkflowRuntime<Request>,
    event: WorkflowLambdaEvent<Request>,
    workflow: S,
) -> Result<SqsBatchResponse, Error>
where
    Request: DeserializeOwned + Serialize + Clone + InvocationId + Send + Debug,
    Response: Serialize + Debug + Send,
    S: Service<WorkflowContext<Request>, Response = Response, Error = WorkflowError> + Clone,
{
    batch_handler(
        move |request: WorkflowEvent<Request>| {
            let engine: WorkflowRuntime<Request> = engine.clone();
            let mut workflow: S = workflow.clone();

            async move {
                let invocation_id: String = request.invocation_id().to_string();
                let ctx: WorkflowContext<Request> = engine.accept(request).await?;

                let workflow_span: Span =
                    tracing::span!(tracing::Level::INFO, "Workflow", %invocation_id);

                let response: Response = workflow.call(ctx).instrument(workflow_span).await?;

                tracing::info!("Completed workflow {:?}", response);

                Ok(())
            }
        },
        event,
    )
    .await
}

pub type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;
