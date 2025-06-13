mod service_name;

use crate::service_name::{NameRequest, NameResponse, NameService};
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, tracing};
use ::model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use service::ServiceRequest;
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::engine::{WorkflowContext, WorkflowEngine};
use workflow::{workflow_handler, WorkflowLambdaEvent};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqsWorkflowResponse {
    sentence: String,
}

async fn workflow_greeter(
    ctx: WorkflowContext<SqsWorkflowRequest>,
) -> Result<SqsWorkflowResponse, WorkflowError> {
    let request: &SqsWorkflowRequest = ctx.request();

    let service_request: ServiceRequest<NameRequest> = ServiceRequest {
        call_id: request.first_letter.clone(),
        inner: NameRequest {
            first_letter: request.first_letter.clone(),
        },
    };

    let response: NameResponse = ctx
        .call(NameService::instance(&ctx.sqs_client()), service_request)
        .await?;

    let sentence: String = format!("Hello {}!", response.name);

    Ok(SqsWorkflowResponse { sentence })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let sqs_client: aws_sdk_sqs::Client =
        aws_sdk_sqs::Client::new(&aws_config::load_defaults(BehaviorVersion::latest()).await);

    let engine: WorkflowEngine<SqsWorkflowRequest> =
        WorkflowEngine::new(Arc::new(InMemoryStateStore::default()), sqs_client);

    lambda_runtime::run(service_fn(
        async |event: WorkflowLambdaEvent<SqsWorkflowRequest>| {
            return workflow_handler(&engine, event, workflow_greeter).await;
        },
    ))
    .await
}
