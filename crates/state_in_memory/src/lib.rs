use async_trait::async_trait;
use model::invocation::WorkflowInvocation;
use model::task::WorkflowTask;
use serde::de::DeserializeOwned;
use serde::Serialize;
use state::{StateError, StateErrorReason, StateOperation, StateStore};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct InMemoryStateStore<Request: DeserializeOwned + Clone> {
    invocations: Arc<Mutex<HashMap<String, Request>>>,
    calls: Arc<Mutex<HashMap<String, WorkflowTask>>>,
}

impl<T: DeserializeOwned + Clone> Default for InMemoryStateStore<T> {
    fn default() -> Self {
        InMemoryStateStore {
            invocations: Arc::new(Mutex::new(Default::default())),
            calls: Arc::new(Mutex::new(Default::default())),
        }
    }
}

#[async_trait]
impl<Request: Serialize + DeserializeOwned + Clone + Send> StateStore<Request>
    for InMemoryStateStore<Request>
{
    async fn put_invocation(
        &self,
        invocation: WorkflowInvocation<Request>,
    ) -> Result<(), StateError> {
        self.invocations
            .lock()
            .unwrap()
            .insert(invocation.invocation_id, invocation.request);

        Ok(())
    }

    async fn get_invocation(&self, invocation_id: &str) -> Result<Request, StateError> {
        let guard = self.invocations.lock().unwrap();
        let state: Request = guard
            .get(invocation_id)
            .ok_or(StateError {
                state_key: invocation_id.to_string(),
                operation: StateOperation::GetInvocation,
                reason: StateErrorReason::MissingEntry,
            })?
            .clone();

        Ok(state)
    }

    async fn put_task(&self, task: WorkflowTask) -> Result<(), StateError> {
        self.calls
            .lock()
            .unwrap()
            .insert(task.task_id.clone(), task);

        Ok(())
    }

    async fn get_task(
        &self,
        _invocation_id: &str,
        task_id: &str,
    ) -> Result<WorkflowTask, StateError> {
        let state = self
            .calls
            .lock()
            .unwrap()
            .get(task_id)
            .ok_or(StateError {
                state_key: task_id.to_string(),
                operation: StateOperation::GetTask,
                reason: StateErrorReason::MissingEntry,
            })
            .map(|state| state.clone())?;

        Ok(state.clone())
    }
}
