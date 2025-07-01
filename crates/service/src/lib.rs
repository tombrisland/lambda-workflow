use model::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Serialize)]
#[serde(tag = "type")]
/// How to re-invoke a workflow from a service perspective.
/// e.g., a QueueUrl or HTTP endpoint
pub enum WorkflowCallback {
    // Expect a Queue URL parameter
    Queue(String),
    // For testing purposes
    Noop
}

const QUEUE_URL: &'static str = "SQS_WORKFLOW_CALLBACK_URL";

impl Default for WorkflowCallback {
    fn default() -> Self {
        // Default to SQS and pull the Queue from the environment
        let queue_url: String = std::env::var(QUEUE_URL)
            .expect(format!("Missing {} environment variable", QUEUE_URL).as_str());
        
        WorkflowCallback::Queue(queue_url)
    }
}

#[derive(Serialize)]
pub struct ServiceRequest<Request: serde::Serialize> {
    pub invocation_id: String,
    // The task id is used as an idempotency key
    pub task_id: String,
    // Callback to allow the service to respond
    pub callback: WorkflowCallback,
    pub payload: Request,
}

impl<Request: serde::Serialize + TaskId> ServiceRequest<Request> {
    pub fn new(payload: Request, invocation_id: String, callback: WorkflowCallback) -> Self {
        let task_id: String = payload.task_id().to_string();

        Self {
            task_id,
            invocation_id,
            callback,
            payload,
        }
    }
}

pub trait TaskId {
    fn task_id(&self) -> &str;
}

/// Name details, Request and Response types for an asynchronous service.
/// The response is returned using a callback mechanism over SQS.
/// The trait `CallableService` is implemented for any implementation of this.
pub trait Service<Request, Response>: Clone
where
    Request: Serialize + TaskId,
    Response: DeserializeOwned + TaskId,
{
    fn name(&self) -> &'static str;

    /// The dispatcher implementation with which to make the request
    /// Most common is an SqsDispatcher implementation
    fn dispatcher(&self) -> impl MessageDispatcher<ServiceRequest<Request>>;
}

pub trait MessageDispatcher<Request>
where
    Request: Serialize,
{
    fn send_message(&self, payload: Request) -> impl Future<Output = Result<(), Error>>;
}

/// Errors arising from parsing state.
#[derive(Debug)]
pub enum ServiceError {
    // The service returned an invalid response
    BadResponse(String),
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for ServiceError {}
