use serde::Serialize;
use serde::de::DeserializeOwned;

/// This id is used for tracing and storage.
pub trait InvocationId {
    fn invocation_id(&self) -> &str;
}

#[derive(Serialize)]
pub struct WorkflowInvocation<Request>
where
    Request: Clone + Serialize + DeserializeOwned,
{
    pub invocation_id: String,
    pub request: Request,
}
