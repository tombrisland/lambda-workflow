use async_trait::async_trait;
use model::CallState;
use serde::de::DeserializeOwned;
use state::{StateError, StateStore};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct InMemoryStateStore<Request: DeserializeOwned + Clone> {
    invocations: Arc<Mutex<HashMap<String, Request>>>,
    calls: Arc<Mutex<HashMap<String, CallState>>>,
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
impl<T: DeserializeOwned + Clone + Send> StateStore<T> for InMemoryStateStore<T> {
    async fn put_invocation(&self, invocation_id: &str, request: T) -> Result<(), StateError> {
        self.invocations
            .lock()
            .unwrap()
            .insert(invocation_id.to_string(), request);

        Ok(())
    }

    async fn get_invocation(&self, invocation_id: &str) -> Result<T, StateError> {
        let guard = self.invocations.lock().unwrap();
        let state: T = guard
            .get(invocation_id)
            .ok_or(StateError::MissingState(invocation_id.to_string()))?
            .clone();

        Ok(state)
    }

    async fn put_call(
        &self,
        _invocation_id: &str,
        call_id: &str,
        state: CallState,
    ) -> Result<(), StateError> {
        self.calls
            .lock()
            .unwrap()
            .insert(call_id.parse().unwrap(), state);

        Ok(())
    }

    async fn get_call(&self, _invocation_id: &str, call_id: &str) -> Result<CallState, StateError> {
        let state = self
            .calls
            .lock()
            .unwrap()
            .get(call_id)
            .ok_or(StateError::MissingState(call_id.to_string()))
            .map(|state| state.clone())?;

        Ok(state.clone())
    }
}
