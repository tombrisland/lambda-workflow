use crate::model::{WorkflowError, WorkflowId};
use crate::service::DummyService;
use crate::workflow_engine::WorkflowContext;
use serde_derive::{Deserialize, Serialize};

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

pub(crate) async fn workflow_example(
    ctx: WorkflowContext<RequestExample>,
) -> Result<ResponseExample, WorkflowError> {
    let request: &RequestExample = ctx.get_request();

    let result: String = ctx.call(DummyService {}, request.item_id.as_str()).await?;

    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result,
    })
}
