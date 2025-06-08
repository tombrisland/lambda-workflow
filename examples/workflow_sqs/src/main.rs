mod model;
mod service_greeter;

use crate::service_greeter::{GreeterRequest, GreeterService};
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, tracing};
use ::model::{Error, WorkflowError, WorkflowId};
use serde::{Deserialize, Serialize};
use service::ServiceRequest;
use state_in_memory::InMemoryStateStore;
use std::sync::Arc;
use workflow::engine::{WorkflowContext, WorkflowEngine};
use workflow::{workflow_handler, WorkflowLambdaEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqsWorkflowRequest {
    pub(crate) name: String,
}

impl WorkflowId for SqsWorkflowRequest {
    fn workflow_id(&self) -> &str {
        &self.name
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

    let result: String = ctx
        .call(
            GreeterService::new(&ctx.sqs_client()),
            ServiceRequest {
                call_id: request.name.clone(),
                inner: GreeterRequest {
                    name: request.name.clone(),
                },
            },
        )
        .await?;

    Ok(SqsWorkflowResponse { sentence: result })
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