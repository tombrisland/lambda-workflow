pub use crate::context::WorkflowContext;
use model::invocation::WorkflowInvocation;
use model::{Error, InvocationId, WorkflowEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use state::StateStore;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct WorkflowRuntime<
    WorkflowRequest: DeserializeOwned + Clone + InvocationId,
    WorkflowResponse: Serialize,
> {
    state_store: Arc<dyn StateStore<WorkflowRequest>>,
    _response: PhantomData<WorkflowResponse>,
}

impl<
    WorkflowRequest: Serialize + DeserializeOwned + Clone + InvocationId + Send,
    WorkflowResponse: Serialize,
> WorkflowRuntime<WorkflowRequest, WorkflowResponse>
{
    pub fn new(
        state_store: Arc<dyn StateStore<WorkflowRequest>>,
    ) -> WorkflowRuntime<WorkflowRequest, WorkflowResponse> {
        WorkflowRuntime {
            state_store,
            _response: Default::default(),
        }
    }

    pub async fn accept(
        &self,
        event: WorkflowEvent<WorkflowRequest>,
    ) -> Result<WorkflowContext<WorkflowRequest>, Error> {
        let request: WorkflowRequest = match event {
            WorkflowEvent::Request(request) => {
                // Create a new invocation record
                let invocation: WorkflowInvocation<WorkflowRequest> = WorkflowInvocation {
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

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }
}

pub struct OutputX {
    sqs_client: aws_sdk_sqs::Client,
    queue_url: String,
}

const QUEUE_URL: &'static str = "SQS_INPUT_QUEUE_URL";

impl OutputX {
    pub fn new(sqs_client: aws_sdk_sqs::Client) -> Self {
        let queue_url: String = std::env::var(QUEUE_URL)
            .expect(format!("Missing {} environment variable", QUEUE_URL).as_str());
        
        Self {
            sqs_client,
            queue_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::context::WorkflowContext;
    use crate::runtime::WorkflowRuntime;
    use model::invocation::WorkflowInvocation;
    use model::task::{CompletedTask, WorkflowTask};
    use model::{Error, InvocationId, WorkflowEvent};
    use state::StateStore;
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use test_utils::TestRequest;

    #[tokio::test]
    async fn runtime_initialises_invocation() {
        let runtime: WorkflowRuntime<TestRequest, String> =
            WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()));

        let request_string: String = "test 1".to_string();
        let request: WorkflowEvent<TestRequest> =
            WorkflowEvent::Request(request_string.clone().into());

        let context: WorkflowContext<TestRequest> = runtime
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

    #[tokio::test]
    async fn runtime_fails_updating_missing_invocation() {
        let runtime: WorkflowRuntime<TestRequest, String> =
            WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()));

        let request_string: String = "test 1".to_string();
        let request: WorkflowEvent<TestRequest> = WorkflowEvent::Update(CompletedTask {
            invocation_id: request_string.clone(),
            task_id: request_string,
            payload: Default::default(),
        });

        // Should fail because the request doesn't exist
        let result: Result<WorkflowContext<TestRequest>, Error> = runtime.accept(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn runtime_updates_existing_invocation() {
        let state_store: Arc<InMemoryStateStore<TestRequest>> =
            Arc::new(InMemoryStateStore::default());

        let runtime: WorkflowRuntime<TestRequest, String> =
            WorkflowRuntime::new(state_store.clone());

        let invocation_id: String = "invocation 1".to_string();
        let task_id: String = "task 1".to_string();

        let request: WorkflowEvent<TestRequest> = WorkflowEvent::Update(CompletedTask {
            invocation_id: invocation_id.clone(),
            task_id: task_id.clone(),
            payload: Default::default(),
        });

        // First store an invocation in the state store
        state_store
            .put_invocation(WorkflowInvocation {
                invocation_id: invocation_id.clone(),
                request: TestRequest(invocation_id.clone()),
            })
            .await
            .expect("Should be able to store invocation");

        runtime
            .accept(request)
            .await
            .expect("Initial request should succeed");

        // Should store the value of the completed task
        let task: WorkflowTask = state_store
            // Stored under the task id
            .get_task(invocation_id.as_str(), &task_id)
            .await
            .expect("Task should exist in state store");

        assert_eq!(task_id, task.task_id);
    }
}
