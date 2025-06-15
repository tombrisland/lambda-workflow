use serde::{Deserialize, Serialize};

/// This id is used for tracing and storage.
pub trait InvocationId {
    fn invocation_id(&self) -> &str;
}

#[derive(Serialize, Deserialize)]
pub struct WorkflowInvocation<Request>
where
    Request: Clone + Serialize,
{
    pub invocation_id: String,
    pub request: Request,
}
