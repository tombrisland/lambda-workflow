use lambda_runtime::tracing;
use model::invocation::WorkflowInvocation;
use model::task::{WorkflowTask, WorkflowTaskState};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use service::{CallableService, ServiceError, ServiceRequest};
use state::StateStore;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkflowRuntime<T: DeserializeOwned + Clone + InvocationId> {
    state_store: Arc<dyn StateStore<T>>,
    sqs_client: Rc<aws_sdk_sqs::Client>,
}

impl<Request: Serialize + DeserializeOwned + Clone + InvocationId + Send> WorkflowRuntime<Request> {
    pub fn new(
        state_store: Arc<dyn StateStore<Request>>,
        sqs_client: aws_sdk_sqs::Client,
    ) -> WorkflowRuntime<Request> {
        WorkflowRuntime {
            state_store,
            sqs_client: Rc::new(sqs_client),
        }
    }

    pub async fn accept(
        &self,
        event: WorkflowEvent<Request>,
    ) -> Result<WorkflowContext<Request>, Error> {
        let request: Request = match event {
            WorkflowEvent::Request(request) => {
                // Create a new invocation record
                let invocation: WorkflowInvocation<Request> = WorkflowInvocation {
                    invocation_id: request.invocation_id().to_string(),
                    request: request.clone(),
                };

                self.state_store.put_invocation(invocation).await?;

                Ok(request)
            }
            WorkflowEvent::Update(task) => {
                let invocation_id: &str = &task.invocation_id.clone();
                // Update the state with any calls
                self.state_store.put_task(task.into()).await?;

                self.state_store.get_invocation(invocation_id).await
            }
        }?;

        Ok(WorkflowContext::new(
            request,
            self.state_store.clone(),
            self.sqs_client.clone(),
        ))
    }
}

pub struct WorkflowContext<T: DeserializeOwned + Clone + InvocationId> {
    request: T,
    state_store: Arc<dyn StateStore<T>>,
    sqs_client: Rc<aws_sdk_sqs::Client>,
}

impl<T: DeserializeOwned + Clone + InvocationId + Send + serde::Serialize> WorkflowContext<T> {
    pub fn new(
        request: T,
        state_store: Arc<dyn StateStore<T>>,
        sqs_client: Rc<aws_sdk_sqs::Client>,
    ) -> Self {
        WorkflowContext {
            request,
            state_store,
            sqs_client,
        }
    }

    pub fn request(&self) -> &T {
        &self.request
    }

    pub fn sqs_client(&self) -> &Rc<aws_sdk_sqs::Client> {
        &self.sqs_client
    }

    /// Call an async service which won't return immediately.
    /// This will suspend execution until a response is received.
    pub async fn call<Request: serde::Serialize, Response: DeserializeOwned>(
        &self,
        service: impl CallableService<Request, Response>,
        request: ServiceRequest<Request>,
    ) -> Result<Response, WorkflowError> {
        let invocation_id: &str = self.request.invocation_id();
        let task_id: &str = request.call_id.as_str();

        tracing::debug!(
            service = service.name(),
            task_id = task_id,
            "Service call"
        );

        // Check if the result is already available in state
        if let Ok(task) = self.state_store.get_task(invocation_id, task_id).await {
            return match task.state {
                // Suspend if it's not available
                WorkflowTaskState::Started => {
                    tracing::debug!("Suspending as task invoked already");
                    
                    Err(WorkflowError::Suspended)
                },
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

#[cfg(test)]
mod tests {
    use crate::runtime::{WorkflowContext, WorkflowRuntime};
    use aws_sdk_sqs::operation::send_message::SendMessageOutput;
    use aws_smithy_mocks::mock_client;
    use model::{InvocationId, WorkflowEvent};
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use test_utils::TestRequest;

    #[tokio::test]
    async fn test_engine_initialises_request() {
        let send_message_rule = aws_smithy_mocks::mock!(aws_sdk_sqs::Client::send_message)
            .then_output(|| {
                let output = SendMessageOutput::builder();

                output.build()
            });

        let sqs_client: aws_sdk_sqs::Client = mock_client!(aws_sdk_sqs, [&send_message_rule]);
        let engine: WorkflowRuntime<TestRequest> =
            WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), sqs_client);

        let request_string: String = "test 1".to_string();
        let request: WorkflowEvent<TestRequest> =
            WorkflowEvent::Request(request_string.clone().into());

        let context: WorkflowContext<TestRequest> = engine
            .accept(request)
            .await
            .expect("Initial request should succeed");

        // Invocation should be stored in the state store
        let invocation: TestRequest = context
            .state_store
            .get_invocation(&request_string)
            .await
            .expect("Invocation should exist in state store");
        let invocation_id: String = invocation.invocation_id().to_string();

        assert_eq!(request_string, invocation_id);
    }
}
