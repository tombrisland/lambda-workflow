pub use crate::batch_handler::SqsBatchPublisher;
pub use crate::context::WorkflowContext;
use model::env::{WORKFLOW_INPUT_QUEUE_URL, WORKFLOW_OUTPUT_QUEUE_URL};
use model::invocation::WorkflowInvocation;
use model::{Error, InvocationId, WorkflowEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use state::StateStore;
use std::sync::Arc;

pub struct WorkflowRuntime<
    WorkflowRequest: DeserializeOwned + Clone + InvocationId,
    WorkflowResponse: Serialize,
> {
    state_store: Arc<dyn StateStore<WorkflowRequest>>,
    // How a service re-invokes this workflow
    input_queue_url: String,
    // Output message client
    pub(crate) publisher: SqsBatchPublisher<WorkflowResponse>,
}

impl<
    WorkflowRequest: Serialize + DeserializeOwned + Clone + InvocationId + Send,
    WorkflowResponse: Serialize,
> WorkflowRuntime<WorkflowRequest, WorkflowResponse>
{
    /// Create a new `WorkflowRuntime` supplying all arguments.
    pub fn new(
        state_store: Arc<dyn StateStore<WorkflowRequest>>,
        sqs: aws_sdk_sqs::Client,
    ) -> WorkflowRuntime<WorkflowRequest, WorkflowResponse> {
        // Pull queues from the environment by default
        let input_queue_url: String = std::env::var(WORKFLOW_INPUT_QUEUE_URL)
            .expect(format!("Missing {} environment variable", WORKFLOW_INPUT_QUEUE_URL).as_str());
        let output_queue_url: String = std::env::var(WORKFLOW_OUTPUT_QUEUE_URL)
            .expect(format!("Missing {} environment variable", WORKFLOW_OUTPUT_QUEUE_URL).as_str());

        WorkflowRuntime {
            state_store,
            input_queue_url,
            publisher: SqsBatchPublisher::new(sqs, output_queue_url),
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

        Ok(WorkflowContext::new(
            request,
            self.state_store.clone(),
            self.input_queue_url.clone(),
        ))
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
    use test_utils::{create_mock_sqs_client, setup_default_env, TestStr};

    #[tokio::test]
    async fn runtime_initialises_invocation() {
        setup_default_env();

        let runtime: WorkflowRuntime<TestStr, String> = WorkflowRuntime::new(
            Arc::new(InMemoryStateStore::default()),
            create_mock_sqs_client(),
        );

        let request_string: String = "test 1".to_string();
        let request: WorkflowEvent<TestStr> =
            WorkflowEvent::Request(request_string.clone().into());

        let context: WorkflowContext<TestStr> = runtime
            .accept(request)
            .await
            .expect("Initial request should succeed");

        // Invocation should be stored in the state store
        let invocation: TestStr = context
            .state_store
            .get_invocation(&request_string)
            .await
            .expect("Invocation should exist in state store");
        let invocation_id: String = invocation.invocation_id().to_string();

        assert_eq!(request_string, invocation_id);
    }

    #[tokio::test]
    async fn runtime_fails_updating_missing_invocation() {
        setup_default_env();

        let runtime: WorkflowRuntime<TestStr, String> = WorkflowRuntime::new(
            Arc::new(InMemoryStateStore::default()),
            create_mock_sqs_client(),
        );

        let request_string: String = "test 1".to_string();
        let request: WorkflowEvent<TestStr> = WorkflowEvent::Update(CompletedTask {
            invocation_id: request_string.clone(),
            task_id: request_string,
            payload: Default::default(),
        });

        // Should fail because the request doesn't exist
        let result: Result<WorkflowContext<TestStr>, Error> = runtime.accept(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn runtime_updates_existing_invocation() {
        setup_default_env();

        let state_store: Arc<InMemoryStateStore<TestStr>> =
            Arc::new(InMemoryStateStore::default());

        let runtime: WorkflowRuntime<TestStr, String> =
            WorkflowRuntime::new(state_store.clone(), create_mock_sqs_client());

        let invocation_id: String = "invocation 1".to_string();
        let task_id: String = "task 1".to_string();

        let request: WorkflowEvent<TestStr> = WorkflowEvent::Update(CompletedTask {
            invocation_id: invocation_id.clone(),
            task_id: task_id.clone(),
            payload: Default::default(),
        });

        // First store an invocation in the state store
        state_store
            .put_invocation(WorkflowInvocation {
                invocation_id: invocation_id.clone(),
                request: TestStr(invocation_id.clone()),
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
