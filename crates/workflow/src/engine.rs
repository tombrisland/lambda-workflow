use model::task::{CompletedTask, RunningTask, WorkflowTask};
use model::{Error, InvocationId, WorkflowError, WorkflowEvent};
use serde::de::DeserializeOwned;
use service::{Service, ServiceError, ServiceRequest};
use state::StateStore;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkflowEngine<T: DeserializeOwned + Clone + InvocationId> {
    state_store: Arc<dyn StateStore<T>>,
    sqs_client: Rc<aws_sdk_sqs::Client>,
}

impl<Request: DeserializeOwned + Clone + InvocationId + Send> WorkflowEngine<Request> {
    pub fn new(
        state_store: Arc<dyn StateStore<Request>>,
        sqs_client: aws_sdk_sqs::Client,
    ) -> WorkflowEngine<Request> {
        WorkflowEngine {
            state_store,
            sqs_client: Rc::new(sqs_client),
        }
    }

    pub async fn accept(
        &self,
        event: WorkflowEvent<Request>,
    ) -> Result<WorkflowContext<Request>, Error> {
        match event {
            WorkflowEvent::Request(request) => self.create_ctx(request).await,
            WorkflowEvent::Update(state) => self.update_ctx(state).await,
        }
    }

    async fn create_ctx(&self, request: Request) -> Result<WorkflowContext<Request>, Error> {
        self.state_store
            .put_invocation(request.invocation_id(), request.clone())
            .await?;

        Ok(WorkflowContext::new(
            request,
            self.state_store.clone(),
            self.sqs_client.clone(),
        ))
    }

    async fn update_ctx(
        &self,
        call_result: CompletedTask,
    ) -> Result<WorkflowContext<Request>, Error> {
        let invocation_id: &str = &call_result.invocation_id.clone();
        let task_id: &str = &call_result.task_id.clone();

        let request: Request = self.state_store.get_invocation(invocation_id).await?;

        // Update the state with any calls
        self.state_store
            .put_call(invocation_id, task_id, WorkflowTask::Completed(call_result))
            .await?;

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

impl<T: DeserializeOwned + Clone + InvocationId + Send> WorkflowContext<T> {
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
        service: impl Service<Request, Response>,
        request: ServiceRequest<Request>,
    ) -> Result<Response, WorkflowError> {
        let invocation_id: &str = self.request.invocation_id();
        let task_id: &str = request.call_id.as_str();

        // Check if the result is already available in state
        if let Ok(state) = self.state_store.get_call(invocation_id, task_id).await {
            return match state {
                // Suspend if it's not available
                WorkflowTask::Running(_) => Err(WorkflowError::Suspended),
                // Return the completed result
                WorkflowTask::Completed(task) => {
                    // Try and mutate the result into the expected value
                    serde_json::from_value(task.payload.clone()).map_err(|err| {
                        WorkflowError::Error(ServiceError::BadResponse(err.to_string()).into())
                    })
                }
            };
        }

        let running_task: WorkflowTask = WorkflowTask::Running(RunningTask {
            invocation_id: invocation_id.to_string(),
            task_id: task_id.to_string(),
        });

        // Set the state running and then call the service
        self.state_store
            .put_call(invocation_id, task_id, running_task)
            .await
            .map_err(|err| WorkflowError::Error(err.into()))?;
        service.call(request.inner).await?;

        Err(WorkflowError::Suspended)
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{WorkflowContext, WorkflowEngine};
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
        let engine: WorkflowEngine<TestRequest> =
            WorkflowEngine::new(Arc::new(InMemoryStateStore::default()), sqs_client);

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
