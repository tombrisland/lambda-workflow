use crate::in_memory_state::InMemoryStateStore;
use crate::model::{CallResult, CallState, WorkflowError, WorkflowEvent, WorkflowId};
use crate::service::AsyncService;
use crate::state::StateStore;
use lambda_runtime::Error;
use serde::Deserialize;
use std::sync::Arc;

pub struct WorkflowEngine<T: Deserialize<'static> + Clone + WorkflowId> {
    state_store: Arc<dyn StateStore<T>>,
}

impl<T: Deserialize<'static> + Clone + WorkflowId + 'static> WorkflowEngine<T> {
    pub fn new() -> WorkflowEngine<T> {
        WorkflowEngine {
            state_store: Arc::new(InMemoryStateStore::default()),
        }
    }

    pub fn accept(&self, event: WorkflowEvent<T>) -> Result<WorkflowContext<T>, Error> {
        match event {
            WorkflowEvent::Request(request) => self.create_ctx(request),
            WorkflowEvent::Update(state) => self.update_ctx(state),
        }
    }

    fn create_ctx(&self, request: T) -> Result<WorkflowContext<T>, Error> {
        self.state_store
            .put_invocation(request.workflow_id(), request.clone())?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }

    fn update_ctx(&self, call_result: CallResult) -> Result<WorkflowContext<T>, Error> {
        let workflow_id: &str = &call_result.workflow_id;
        let call_id: &str = &call_result.call_id.clone();

        let request: T = self
            .state_store
            .get_invocation(workflow_id)
            .ok_or(format!("Failed to get workflow id {}", workflow_id).as_str())?;

        // Update the state with any calls
        self.state_store
            .put_call(call_id, CallState::Completed(call_result))?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }
}

pub struct WorkflowContext<T: Deserialize<'static> + Clone + WorkflowId> {
    request: T,
    state_store: Arc<dyn StateStore<T>>,
}

impl<T: Deserialize<'static> + Clone + WorkflowId> WorkflowContext<T> {
    pub fn new(request: T, state_store: Arc<dyn StateStore<T>>) -> Self {
        WorkflowContext {
            request,
            state_store,
        }
    }

    pub fn get_request(&self) -> &T {
        &self.request
    }

    pub async fn call(
        &self,
        service: impl AsyncService<String>,
        call_id: &str,
    ) -> Result<String, WorkflowError> {
        // Check if the result is already available in state
        if let Some(state) = self.state_store.get_call(call_id) {
            return match state {
                // Suspend if it's not available
                CallState::Running => Err(WorkflowError::Suspended),
                // Return the completed result
                CallState::Completed(result) => Ok(result.value.clone()),
            };
        }

        // Set the state running and then call the service
        self.state_store
            // TODO idempotency?
            .put_call(call_id, CallState::Running)?;
        service.call(call_id.to_string()).await?;

        Err(WorkflowError::Suspended)
    }
}

// pub async fn init_context<T: Serialize>(ctx: WorkflowContext<T>, event: ) -> Result<String, WorkflowError> {
//     let context = WorkflowContext::new()
// }
