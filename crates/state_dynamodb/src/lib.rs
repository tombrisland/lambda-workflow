use async_trait::async_trait;
use aws_sdk_dynamodb::operation::get_item::GetItemOutput;
use aws_sdk_dynamodb::types::AttributeValue;
use model::invocation::WorkflowInvocation;
use model::task::WorkflowTask;
use serde::Serialize;
use serde::de::DeserializeOwned;
use state::StateErrorReason::{BackendFailure, BadState, MissingEntry};
use state::StateOperation::{GetInvocation, GetTask, PutInvocation, PutTask};
use state::{StateError, StateStore};
use std::collections::HashMap;
use std::marker::PhantomData;

const INVOCATION_ID: &str = "invocation_id";
const TASK_ID: &str = "invocation_id";

pub struct DynamoDbStateStore<Request> {
    table_name: String,
    dynamodb_client: aws_sdk_dynamodb::Client,
    // Hold expected request type
    phantom_data: PhantomData<Request>,
}

#[async_trait]
impl<Request: Serialize + DeserializeOwned + Clone + Send + Sync> StateStore<Request>
    for DynamoDbStateStore<Request>
{
    async fn put_invocation(
        &self,
        invocation: WorkflowInvocation<Request>,
    ) -> Result<(), StateError> {
        let item: HashMap<String, AttributeValue> =
            serde_dynamo::to_item(&invocation).map_err(|err| {
                StateError::new(
                    invocation.invocation_id.clone(),
                    PutInvocation,
                    BadState(err.to_string()),
                )
            })?;

        self.dynamodb_client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|err| {
                StateError::new(
                    invocation.invocation_id.to_string(),
                    PutTask,
                    BackendFailure(err.into()),
                )
            })?;

        Ok(())
    }

    async fn get_invocation(&self, invocation_id: &str) -> Result<Request, StateError> {
        let output: GetItemOutput = self
            .dynamodb_client
            .get_item()
            .key(INVOCATION_ID, AttributeValue::S(invocation_id.to_string()))
            .send()
            .await
            .map_err(|err| {
                StateError::new(
                    invocation_id.to_string(),
                    GetInvocation,
                    BackendFailure(err.into()),
                )
            })?;

        let item: HashMap<String, AttributeValue> = output.item.ok_or_else(|| {
            StateError::new(invocation_id.to_string(), GetInvocation, MissingEntry)
        })?;

        serde_dynamo::from_item(item).map_err(|err| {
            StateError::new(
                invocation_id.to_string(),
                GetInvocation,
                BadState(err.to_string()),
            )
        })
    }

    async fn put_task(&self, task: WorkflowTask) -> Result<(), StateError> {
        let item: HashMap<String, AttributeValue> =
            serde_dynamo::to_item(&task).map_err(|err| {
                StateError::new(
                    task.invocation_id.clone(),
                    PutTask,
                    BadState(err.to_string()),
                )
            })?;

        self.dynamodb_client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|err| {
                StateError::new(
                    task.task_id.to_string(),
                    PutTask,
                    BackendFailure(err.into()),
                )
            })?;

        Ok(())
    }

    async fn get_task(
        &self,
        invocation_id: &str,
        task_id: &str,
    ) -> Result<WorkflowTask, StateError> {
        let output: GetItemOutput = self
            .dynamodb_client
            .get_item()
            .key(INVOCATION_ID, AttributeValue::S(invocation_id.to_string()))
            .key(TASK_ID, AttributeValue::S(task_id.to_string()))
            .send()
            .await
            .map_err(|err| {
                StateError::new(task_id.to_string(), GetTask, BackendFailure(err.into()))
            })?;

        let item: HashMap<String, AttributeValue> = output
            .item
            .ok_or_else(|| StateError::new(invocation_id.to_string(), GetTask, MissingEntry))?;

        serde_dynamo::from_item(item).map_err(|err| {
            StateError::new(
                invocation_id.to_string(),
                GetTask,
                BadState(err.to_string()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_invocation_put() {}
}
