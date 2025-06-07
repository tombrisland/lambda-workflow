use crate::model::CallState;
use crate::state::StateStore;
use lambda_runtime::Error;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct InMemoryStateStore<Request: Deserialize<'static> + Clone> {
    invocations: Arc<Mutex<HashMap<String, Request>>>,
    calls: Arc<Mutex<HashMap<String, CallState>>>,
}

impl<T: Deserialize<'static> + Clone> Default for InMemoryStateStore<T> {
    fn default() -> Self {
        InMemoryStateStore {
            invocations: Arc::new(Mutex::new(Default::default())),
            calls: Arc::new(Mutex::new(Default::default())),
        }
    }
}

impl<T: Deserialize<'static> + Clone> StateStore<T> for InMemoryStateStore<T> {
    fn put_invocation(&self, workflow_id: &str, request: T) -> Result<(), Error> {
        self.invocations
            .lock()
            .unwrap()
            .insert(workflow_id.to_string(), request);

        Ok(())
    }

    fn get_invocation(&self, workflow_id: &str) -> Option<T> {
        let guard = self.invocations.lock().unwrap();
        let state: T = guard.get(workflow_id)?.clone();

        Some(state)
    }

    fn put_call(&self, call_id: &str, state: CallState) -> Result<(), Error> {
        self.calls
            .lock()
            .unwrap()
            .insert(call_id.parse().unwrap(), state);

        Ok(())
    }

    fn get_call(&self, call_id: &str) -> Option<CallState> {
        let state = self
            .calls
            .lock()
            .unwrap()
            .get(call_id)
            .map(|state| state.clone())?;

        Some(state.clone())
    }
}
