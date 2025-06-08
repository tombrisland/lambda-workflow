use serde::de::DeserializeOwned;
use std::sync::Arc;
use model::{CallResult, CallState, Error, WorkflowError, WorkflowEvent, WorkflowId};
use service::AsyncService;
use state::StateStore;

#[derive(Clone)]
pub struct WorkflowEngine<T: DeserializeOwned + Clone + WorkflowId> {
    state_store: Arc<dyn StateStore<T>>,
}

impl<Request: DeserializeOwned + Clone + WorkflowId + 'static> WorkflowEngine<Request> {
    pub fn new(state_store: Arc<dyn StateStore<Request>>) -> WorkflowEngine<Request> {
        WorkflowEngine {
            state_store,
        }
    }

    pub fn accept(&self, event: WorkflowEvent<Request>) -> Result<WorkflowContext<Request>, Error> {
        match event {
            WorkflowEvent::Request(request) => self.create_ctx(request),
            WorkflowEvent::Update(state) => self.update_ctx(state),
        }
    }

    fn create_ctx(&self, request: Request) -> Result<WorkflowContext<Request>, Error> {
        self.state_store
            .put_invocation(request.workflow_id(), request.clone())?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }

    fn update_ctx(&self, call_result: CallResult) -> Result<WorkflowContext<Request>, Error> {
        let workflow_id: &str = &call_result.workflow_id.clone();
        let call_id: &str = &call_result.call_id.clone();

        let request: Request = self
            .state_store
            .get_invocation(workflow_id)
            .ok_or(format!("Failed to get workflow id {}", workflow_id).as_str())?;

        // Update the state with any calls
        self.state_store
            .put_call(workflow_id, call_id, CallState::Completed(call_result))?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }
}

pub struct WorkflowContext<T: DeserializeOwned + Clone + WorkflowId> {
    request: T,
    state_store: Arc<dyn StateStore<T>>,
}

impl<T: DeserializeOwned + Clone + WorkflowId> WorkflowContext<T> {
    pub fn new(request: T, state_store: Arc<dyn StateStore<T>>) -> Self {
        WorkflowContext {
            request,
            state_store,
        }
    }

    pub fn get_request(&self) -> &T {
        &self.request
    }

    /// Call an async service which isn't expected to return immediately.
    /// This will suspend execution until a response is received.
    pub async fn call(
        &self,
        service: impl AsyncService<String>,
        call_id: &str,
    ) -> Result<String, WorkflowError> {
        let workflow_id: &str = self.request.workflow_id();

        // Check if the result is already available in state
        if let Some(state) = self.state_store.get_call(workflow_id, call_id) {
            return match state {
                // Suspend if it's not available
                CallState::Running => Err(WorkflowError::Suspended),
                // Return the completed result
                CallState::Completed(result) => Ok(result.value.clone()),
            };
        }

        // Set the state running and then call the service
        self.state_store
            .put_call(workflow_id, call_id, CallState::Running)?;
        service.call(call_id.to_string()).await?;

        Err(WorkflowError::Suspended)
    }
}