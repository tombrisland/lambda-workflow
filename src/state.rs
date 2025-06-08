use crate::model::CallState;
use lambda_runtime::Error;
use serde::de::DeserializeOwned;

pub trait StateStore<T: DeserializeOwned + Clone> {
    fn put_invocation(&self, workflow_id: &str, request: T) -> Result<(), Error>;
    fn get_invocation(&self, workflow_id: &str) -> Option<T>;
    // fn remove_invocation(&self, workflow_id: &str);

    fn put_call(&self, call_id: &str, state: CallState) -> Result<(), Error>;
    fn get_call(&self, call_id: &str) -> Option<CallState>;
}
