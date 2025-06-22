mod service_name;

use crate::service_name::{NameRequest, NameResponse, NameService};
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, tracing};
use ::model::{Error, InvocationId, WorkflowError};
use serde::{Deserialize, Serialize};
use service::ServiceRequest;
use state_in_memory::InMemoryStateStore;
use std::rc::Rc;
use std::sync::Arc;
use workflow::runtime::{WorkflowContext, WorkflowRuntime};
use workflow::{workflow_fn, WorkflowLambdaEvent};

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
    name_service: &NameService,
) -> Result<SqsWorkflowResponse, WorkflowError> {
    let request: &SqsWorkflowRequest = ctx.request();

    let service_request: ServiceRequest<NameRequest> = ServiceRequest {
        task_id: request.first_letter.clone(),
        payload: NameRequest {
            first_letter: request.first_letter.clone(),
        },
    };

    let response: NameResponse = ctx.call(name_service, service_request).await?;

    let sentence: String = format!("Hello {}!", response.name);

    Ok(SqsWorkflowResponse { sentence })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let sqs_client: Rc<aws_sdk_sqs::Client> = Rc::new(aws_sdk_sqs::Client::new(
        &aws_config::load_defaults(BehaviorVersion::latest()).await,
    ));
    let name_service: NameService = NameService::new(sqs_client.clone());

    let engine: WorkflowRuntime<SqsWorkflowRequest> =
        WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), sqs_client);

    lambda_runtime::run(service_fn(
        async |event: WorkflowLambdaEvent<SqsWorkflowRequest>| {
            return workflow_fn(
                &engine,
                event,
                service_fn(async |ctx: WorkflowContext<SqsWorkflowRequest>| {
                    workflow_greeter(ctx, &name_service).await
                }),
            )
            .await;
        },
    ))
    .await
}
