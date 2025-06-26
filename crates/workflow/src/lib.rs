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
use std::sync::Arc;
use std::task::{Context, Poll};

pub mod batch_handler;
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
pub async fn workflow_fn<Request, Response, S>(
    runtime: WorkflowRuntime<Request>,
    event: WorkflowLambdaEvent<Request>,
    workflow: S,
) -> Result<SqsBatchResponse, Error>
where
    Request: DeserializeOwned + Serialize + Clone + InvocationId + Send + Debug,
    Response: Serialize + Debug + Send,
    S: Service<WorkflowContext<Request>, Response = Response, Error = WorkflowError> + Clone,
{
    handle_sqs_batch(
        move |request: WorkflowEvent<Request>| {
            let engine: WorkflowRuntime<Request> = runtime.clone();
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

/// Custom future for workflow execution
pub struct WorkflowFuture<Request, BatchFuture>
where
    Request: DeserializeOwned + Serialize + Clone + InvocationId + Debug + Send + 'static,
    BatchFuture: Future<Output = Result<SqsBatchResponse, Error>>,
{
    batch_future: BatchFuture,
    _phantom: std::marker::PhantomData<Request>,
}

impl<Request, BatchFuture> Future for WorkflowFuture<Request, BatchFuture>
where
    Request: DeserializeOwned + Serialize + Clone + InvocationId + Debug + Send + 'static,
    BatchFuture: Future<Output = Result<SqsBatchResponse, Error>>,
{
    type Output = Result<SqsBatchResponse, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: We're only projecting to batch_future which is not self-referential
        let batch_future = unsafe { self.map_unchecked_mut(|s| &mut s.batch_future) };
        batch_future.poll(cx)
    }
}

type BatchFuture = Pin<Box<dyn Future<Output = Result<SqsBatchResponse, Error>> + Send>>;

/// Creates a workflow handler function that can be called directly by `lambda_runtime::run()`
// pub fn workflow_fn2<Request, Response, WorkflowFunction, WF>(
//     runtime: WorkflowRuntime<Request>,
//     workflow: WorkflowFunction,
// ) -> impl Fn(WorkflowLambdaEvent<Request>) -> WorkflowFuture<Request, BatchFuture>
// where
//     Request: DeserializeOwned + Serialize + Clone + InvocationId + Debug + Send + 'static,
//     Response: Serialize + Debug,
//     WorkflowFunction: Fn(WorkflowContext<Request>) -> WF + Send + Sync + 'static,
//     WF: Future<Output = Result<Response, WorkflowError>> + Send + 'static,
// {
//     // Create a shared handler that captures both runtime and workflow once
//     let shared_handler = Arc::new(WorkflowHandler {
//         runtime: Arc::new(runtime),
//         workflow: Arc::new(workflow),
//     });
//
//     move |event: WorkflowLambdaEvent<Request>| {
//         let handler = shared_handler.clone();
//
//         let batch_future: BatchFuture = Box::pin(async move {
//             batch_handler(
//                 move |request: WorkflowEvent<Request>| {
//                     let handler = handler.clone();
//
//                     async move { handler.execute(request).await }
//                 },
//                 event,
//             )
//             .await
//         });
//
//         WorkflowFuture {
//             batch_future,
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

// Encapsulate both runtime and workflow in a single struct
pub struct WorkflowHandler<WorkflowRequest: Clone + DeserializeOwned + InvocationId, WorkflowFunction> {
    pub runtime: WorkflowRuntime<WorkflowRequest>,
    pub workflow: WorkflowFunction,
}

impl<WorkflowRequest, WorkflowResponse, WorkflowFunction, WorkflowFuture>
    WorkflowHandler<WorkflowRequest, WorkflowFunction>
where
    WorkflowRequest: DeserializeOwned + Serialize + Clone + InvocationId + Debug + Send,
    WorkflowResponse: Serialize + Debug,
    WorkflowFunction: Fn(WorkflowContext<WorkflowRequest>) -> WorkflowFuture,
    WorkflowFuture: Future<Output = Result<WorkflowResponse, WorkflowError>>,
{
    pub async fn invoke(
        &self,
        request: WorkflowEvent<WorkflowRequest>,
    ) -> Result<WorkflowResponse, WorkflowError> {
        let invocation_id: String = request.invocation_id().to_string();
        let ctx: WorkflowContext<WorkflowRequest> = self.runtime.accept(request).await?;

        let workflow_span: Span = tracing::span!(tracing::Level::INFO, "Workflow", invocation_id);

        let response: WorkflowResponse = (self.workflow)(ctx).instrument(workflow_span).await?;

        tracing::info!("Completed workflow {:?}", response);
        Ok(response)
    }
}

pub type WorkflowLambdaEvent<T> = LambdaEvent<WorkflowSqsEvent<T>>;
