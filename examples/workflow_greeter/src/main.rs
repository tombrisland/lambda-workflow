mod service_name;

use crate::service_name::{NameRequest, NameResponse, NameService};
use aws_config::BehaviorVersion;
use lambda_runtime::tracing;
use ::model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::context::WorkflowContext;
use workflow::runtime::WorkflowRuntime;
use workflow::{call, workflow_fn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqsWorkflowRequest {
    pub(crate) request_id: String,
    pub(crate) first_letter: String,
}

impl InvocationId for SqsWorkflowRequest {
    fn invocation_id(&self) -> &str {
        &self.request_id
    }
}

#[derive(Debug, Clone, Serialize)]
struct SqsWorkflowResponse {
    sentence: String,
}

async fn workflow_greeter(
    ctx: WorkflowContext<SqsWorkflowRequest>,
    name_service: NameService,
) -> Result<SqsWorkflowResponse, WorkflowError> {
    let request: &SqsWorkflowRequest = ctx.request();

    let name_request: NameRequest = NameRequest {
        first_letter: request.first_letter.clone(),
    };

    // Make two calls to name service for first and last name
    let responses: Vec<NameResponse> = futures::future::join_all([
        // Call both services in parallel
        call!(ctx, &name_service, name_request.clone()),
        call!(ctx, &name_service, name_request.clone()),
    ])
    .await
    // Collect from Vec<Result<T, E>> into Result<Vec<T>, E>
    .into_iter()
    .collect::<Result<Vec<NameResponse>, WorkflowError>>()?;

    let name: String = responses
        .into_iter()
        .map(|response| response.name)
        .collect::<Vec<String>>()
        .join(" ");

    let sentence: String = format!("Hello {}!", name);

    Ok(SqsWorkflowResponse { sentence })
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let sqs: aws_sdk_sqs::Client =
        aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);
    let name_service: NameService = NameService::new(sqs.clone());

    let runtime: WorkflowRuntime<SqsWorkflowRequest, SqsWorkflowResponse> =
        WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), sqs);

    lambda_runtime::run(workflow_fn(
        &runtime,
        |ctx: WorkflowContext<SqsWorkflowRequest>| workflow_greeter(ctx, name_service.clone()),
    ))
    .await
}
