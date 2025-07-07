use lambda_runtime::tracing;
use model::task::{WorkflowTask, WorkflowTaskState};
use model::{InvocationId, WorkflowError};
use serde::de::DeserializeOwned;
use service::MessageDispatcher;
use service::{Service, ServiceError, ServiceRequest, TaskId};
use state::StateStore;
use std::sync::Arc;

pub struct WorkflowContext<T: DeserializeOwned + Clone + InvocationId> {
    request: T,
    pub(crate) state_store: Arc<dyn StateStore<T>>,
    callback_queue_url: String,
}

struct CallInFlight {
    request: String,
    dispatcher: Box<dyn MessageDispatcher>
}

impl<T: DeserializeOwned + Clone + InvocationId + Send + serde::Serialize> WorkflowContext<T> {
    pub fn new(
        request: T,
        state_store: Arc<dyn StateStore<T>>,
        callback_queue_url: String,
    ) -> Self {
        WorkflowContext {
            request,
            state_store,
            callback_queue_url,
        }
    }

    pub fn request(&self) -> &T {
        &self.request
    }

    /// Call an async service which won't return immediately.
    /// This will suspend execution until a response is received.
    pub async fn call<Payload: serde::Serialize + TaskId, Response: DeserializeOwned + TaskId>(
        &self,
        service: &impl Service<Payload, Response>,
        payload: Payload,
    ) -> Result<Response, WorkflowError> {
        let invocation_id: &str = self.request.invocation_id();
        let task_id: String = payload.task_id().to_string();

        tracing::debug!(service = service.name(), task_id = task_id, "Service call");

        // Check if the result is already available
        if let Ok(task) = self
            .state_store
            .get_task(invocation_id, task_id.as_str())
            .await
        {
            return match task.state {
                // Task has already started but not completed
                WorkflowTaskState::Started => {
                    tracing::debug!("Task was already started");

                    Err(WorkflowError::Suspended)
                }
                // Task is completed and the result is available
                WorkflowTaskState::Completed(payload) => {
                    tracing::debug!("Task completed and result available");

                    // Try and convert the result into the expected value
                    serde_json::from_value(payload.clone()).map_err(|err| {
                        WorkflowError::Error(ServiceError::BadResponse(err.to_string()).into())
                    })
                }
            };
        }

        let request: ServiceRequest<Payload> = ServiceRequest::new(
            payload,
            invocation_id.to_string(),
            self.callback_queue_url.clone(),
        );
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

        let request: String =
            serde_json::to_string(&request).map_err(|err| WorkflowError::Error(err.into()))?;
        service.dispatcher().send_message(request).await?;

        tracing::debug!("Suspending after invoking task");

        Err(WorkflowError::Suspended)
    }
}
