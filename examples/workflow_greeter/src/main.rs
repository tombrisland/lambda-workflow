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
use workflow::workflow_fn;

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
    let first_name_fut = ctx.call(&name_service, name_request.clone());
    let last_name_fut = ctx.call(&name_service, name_request.clone());

    let first_name: NameResponse = first_name_fut.await?;
    let last_name: NameResponse = last_name_fut.await?;

    let sentence: String = format!("Hello {} {}!", first_name.name, last_name.name);

    Ok(SqsWorkflowResponse { sentence })
}

#[tokio::main]
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
