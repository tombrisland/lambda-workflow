use serde::de::DeserializeOwned;
use ::model::Error;
use model::CallState;

pub trait StateStore<T: DeserializeOwned + Clone> {
    fn put_invocation(&self, invocation_id: &str, request: T) -> Result<(), Error>;
    fn get_invocation(&self, invocation_id: &str) -> Option<T>;
    // fn remove_invocation(&self, workflow_id: &str);

    fn put_call(&self, invocation_id: &str, call_id: &str, state: CallState) -> Result<(), Error>;
    fn get_call(&self, invocation_id: &str, call_id: &str) -> Option<CallState>;
}
