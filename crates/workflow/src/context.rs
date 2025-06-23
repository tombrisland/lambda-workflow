use std::sync::Arc;
use lambda_runtime::tracing;
use serde::de::DeserializeOwned;
use model::{InvocationId, WorkflowError};
use model::task::{WorkflowTask, WorkflowTaskState};
use service::{CallableService, ServiceError, ServiceRequest, TaskId};
use state::StateStore;

const QUEUE_URL: &'static str = "SQS_INPUT_QUEUE_URL";

pub struct WorkflowContext<T: DeserializeOwned + Clone + InvocationId> {
    request: T,
    pub(crate) state_store: Arc<dyn StateStore<T>>,
}

impl<T: DeserializeOwned + Clone + InvocationId + Send + serde::Serialize> WorkflowContext<T> {
    pub fn new(request: T, state_store: Arc<dyn StateStore<T>>) -> Self {
        WorkflowContext {
            request,
            state_store,
        }
    }

    pub fn request(&self) -> &T {
        &self.request
    }

    /// Call an async service which won't return immediately.
    /// This will suspend execution until a response is received.
    pub async fn call<Payload: serde::Serialize + TaskId, Response: DeserializeOwned + TaskId>(
        &self,
        service: &impl CallableService<Payload, Response>,
        payload: Payload,
    ) -> Result<Response, WorkflowError> {
        let invocation_id: &str = self.request.invocation_id();
        let task_id: String = payload.task_id().to_string();

        tracing::debug!(service = service.name(), task_id = task_id, "Service call");

        // Check if the result is already available in state
        if let Ok(task) = self
            .state_store
            .get_task(invocation_id, task_id.as_str())
            .await
        {
            return match task.state {
                // Suspend if it's not available
                WorkflowTaskState::Started => {
                    tracing::debug!("Suspending as task invoked already");

                    Err(WorkflowError::Suspended)
                }
                // Return the completed result
                WorkflowTaskState::Completed(payload) => {
                    tracing::debug!("Got payload from state store");

                    // Try and convert the result into the expected value
                    serde_json::from_value(payload.clone()).map_err(|err| {
                        WorkflowError::Error(ServiceError::BadResponse(err.to_string()).into())
                    })
                }
            };
        }

        // TODO this needs to move elsewhere
        // Can we make it non SQS specific
        let queue_url: String = std::env::var(QUEUE_URL).unwrap_or_default();

        let request: ServiceRequest<Payload> =
            ServiceRequest::new(payload, invocation_id.to_string(), queue_url);
        let running_task: WorkflowTask = WorkflowTask {
            invocation_id: invocation_id.to_string(),
            task_id: task_id.to_string(),
            state: WorkflowTaskState::Started,
        };

        // Set the state running and then call the service
        self.state_store
            .put_task(running_task)
            .await
            .map_err(|err| WorkflowError::Error(err.into()))?;
        service.call(request).await?;

        tracing::debug!("Suspending after invoking task");

        Err(WorkflowError::Suspended)
    }
}