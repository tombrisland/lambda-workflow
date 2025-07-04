use crate::batch_handler::handle_sqs_batch;
use crate::context::WorkflowContext;
use crate::runtime::WorkflowRuntime;
use aws_lambda_events::sqs::SqsBatchResponse;
use lambda_runtime::tracing::{Instrument, Span};
use lambda_runtime::{tracing, LambdaEvent, Service};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent, WorkflowSqsEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

mod batch_handler;
pub mod context;
pub mod runtime;

/// The service expects to receive a
/// `WorkflowLambdaEvent<WorkflowRequest>` and returns an `SqsBatchResponse`.
/// Therefore, the function *must* have `ReportBatchItemFailures` set to true.
///
/// Returns a `WorkflowService` that implements the `tower::Service` trait and can be
/// passed directly to `lambda_runtime::run()`.
///
/// # Example
/// ```no_run
/// use lambda_runtime::{LambdaEvent};
/// use workflow::runtime::{WorkflowRuntime, WorkflowContext, SqsBatchPublisher};
/// use state_in_memory::InMemoryStateStore;
/// use service::WorkflowCallback;
/// use workflow::{WorkflowLambdaEvent, workflow_fn};
/// use model::{InvocationId, Error, WorkflowError};
/// use aws_sdk_sqs::config::BehaviorVersion;
/// use serde::{Serialize, Deserialize};
/// use std::sync::Arc;
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
/// #[derive(Clone, Serialize, Debug)]
/// struct ExampleResponse {
///     result: String,
/// }
///
/// async fn workflow_example(
///     ctx: WorkflowContext<ExampleRequest>
/// ) -> Result<ExampleResponse, WorkflowError> {
///     Ok(ExampleResponse { result: "done".to_string() })
/// }
/// 
/// const QUEUE_OUTPUT_URL: &str = "https://sqs.eu-west-1.com/queue";
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let sqs: aws_sdk_sqs::Client = aws_sdk_sqs::Client::new(
///         &aws_config::load_defaults(BehaviorVersion::latest()).await,
///     );
///     
///     let runtime: WorkflowRuntime<ExampleRequest, ExampleResponse> =  WorkflowRuntime::new(
///         Arc::new(InMemoryStateStore::default()),
///         WorkflowCallback::default(),
///         SqsBatchPublisher::new(sqs, QUEUE_OUTPUT_URL.into())
///     );
///
///     lambda_runtime::run(workflow_fn(&runtime, workflow_example)).await
/// }
/// ```
pub fn workflow_fn<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, WorkflowFunction>(
    runtime: &'a WorkflowRuntime<WorkflowRequest, WorkflowResponse>,
    workflow: WorkflowFunction,
) -> WorkflowService<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, WorkflowFunction>
where
    WorkflowRequest: DeserializeOwned + Clone + InvocationId + Serialize + Debug + Send,
    WorkflowResponse: Clone + Serialize + Debug + 'a,
    WorkflowFuture: Future<Output = Result<WorkflowResponse, WorkflowError>> + 'a,
    WorkflowFunction:
        Fn(WorkflowContext<WorkflowRequest>) -> WorkflowFuture + Send + Sync + Clone + 'a,
{
    WorkflowService {
        runtime,
        workflow,
        _phantom: Default::default(),
    }
}

pub struct WorkflowService<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, WorkflowFunction>
where
    WorkflowRequest: DeserializeOwned + Clone + InvocationId + Serialize + Debug,
    WorkflowResponse: Clone + Serialize + Debug + 'a,
    WorkflowFuture: Future<Output = Result<WorkflowResponse, WorkflowError>> + 'a,
    WorkflowFunction:
        Fn(WorkflowContext<WorkflowRequest>) -> WorkflowFuture + Send + Sync + Clone + 'a,
{
    runtime: &'a WorkflowRuntime<WorkflowRequest, WorkflowResponse>,
    workflow: WorkflowFunction,
    _phantom: std::marker::PhantomData<(WorkflowResponse, WorkflowFuture)>,
}

impl<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, WorkflowFunction>
    Service<WorkflowLambdaEvent<WorkflowRequest>>
    for WorkflowService<'a, WorkflowRequest, WorkflowResponse, WorkflowFuture, WorkflowFunction>
where
    WorkflowRequest: DeserializeOwned + Clone + InvocationId + Serialize + Debug + Send,
    WorkflowResponse: Clone + Serialize + Debug + 'a,
    WorkflowFuture: Future<Output = Result<WorkflowResponse, WorkflowError>> + 'a,
    WorkflowFunction:
        Fn(WorkflowContext<WorkflowRequest>) -> WorkflowFuture + Send + Sync + Clone + 'a,
{
    type Response = SqsBatchResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'a>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: WorkflowLambdaEvent<WorkflowRequest>) -> Self::Future {
        let workflow: WorkflowFunction = self.workflow.clone();
        let runtime: &WorkflowRuntime<WorkflowRequest, WorkflowResponse> = self.runtime;

        // Handler for each message
        let handler = move |request: WorkflowEvent<WorkflowRequest>| {
            let workflow: WorkflowFunction = workflow.clone();
            let invocation_id: String = request.invocation_id().to_string();

            let workflow_span: Span =
                tracing::span!(tracing::Level::INFO, "Workflow", invocation_id);

            async move {
                let ctx: WorkflowContext<WorkflowRequest> = runtime.accept(request).await?;

                tracing::info!("Starting workflow execution");
                let response: WorkflowResponse =
                    workflow(ctx).await.inspect_err(|err: &WorkflowError| {
                        if let WorkflowError::Suspended = err {
                            tracing::info!("Suspending workflow execution")
                        }
                    })?;
                tracing::info!("Completed workflow execution");

                Ok(response)
            }
            .instrument(workflow_span)
        };

        // Operate handler on each message
        Box::pin(handle_sqs_batch(handler, request, self.runtime.publish.clone()))
    }
}

pub type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;
