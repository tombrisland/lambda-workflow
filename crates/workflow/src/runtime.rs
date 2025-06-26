use model::invocation::WorkflowInvocation;
use model::{Error, InvocationId, WorkflowEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;
use state::StateStore;
use std::rc::Rc;
use std::sync::Arc;
use crate::context::WorkflowContext;

#[derive(Clone)]
pub struct WorkflowRuntime<T: DeserializeOwned + Clone + InvocationId> {
    state_store: Arc<dyn StateStore<T>>,
    _sqs_client: Arc<aws_sdk_sqs::Client>,
}

impl<Request: Serialize + DeserializeOwned + Clone + InvocationId + Send> WorkflowRuntime<Request> {
    pub fn new(
        state_store: Arc<dyn StateStore<Request>>,
        sqs_client: Arc<aws_sdk_sqs::Client>,
    ) -> WorkflowRuntime<Request> {
        WorkflowRuntime {
            state_store,
            _sqs_client: sqs_client,
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

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{WorkflowRuntime};
    use aws_sdk_sqs::operation::send_message::SendMessageOutput;
    use aws_smithy_mocks::mock_client;
    use model::{InvocationId, WorkflowEvent};
    use state_in_memory::InMemoryStateStore;
    use std::rc::Rc;
    use std::sync::Arc;
    use test_utils::TestRequest;
    use crate::context::WorkflowContext;

    #[tokio::test]
    async fn test_engine_initialises_request() {
        let send_message_rule = aws_smithy_mocks::mock!(aws_sdk_sqs::Client::send_message)
            .then_output(|| {
                let output = SendMessageOutput::builder();

                output.build()
            });

        let sqs_client: aws_sdk_sqs::Client = mock_client!(aws_sdk_sqs, [&send_message_rule]);
        let engine: WorkflowRuntime<TestRequest> =
            WorkflowRuntime::new(Arc::new(InMemoryStateStore::default()), Arc::new(sqs_client));

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
