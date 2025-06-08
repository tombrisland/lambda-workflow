use lambda_runtime::tracing::log::info;
use serde_derive::{Deserialize, Serialize};
use crate::model::{WorkflowError, WorkflowId};
use crate::service::DummyService;
use crate::workflow_engine::WorkflowContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RequestExample {
    pub(crate) id: String,
    pub(crate) item_id: String,
}

impl WorkflowId for RequestExample {
    fn workflow_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ResponseExample {
    id: String,
    item_id: String,
    payload: String,
}

pub(crate) async fn workflow_example(ctx: WorkflowContext<RequestExample>) -> Result<ResponseExample, WorkflowError> {
    let request: &RequestExample = ctx.get_request();
    info!("Handling request Example");

    info!("Calling external service");

    let result = ctx.call(DummyService {}, request.item_id.as_str()).await?;

    info!("Got response from service {}", result);

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result,
    })
}