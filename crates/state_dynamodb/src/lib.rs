use async_trait::async_trait;
use aws_sdk_dynamodb::config::http::HttpResponse;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::get_item::{GetItemError, GetItemOutput};
use aws_sdk_dynamodb::operation::put_item::{PutItemError, PutItemOutput};
use aws_sdk_dynamodb::types::AttributeValue;
use model::invocation::WorkflowInvocation;
use model::task::WorkflowTask;
use serde::de::DeserializeOwned;
use serde::Serialize;
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
    consistent_read: bool,
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

        self.put_item(item).await.map_err(|err| {
            StateError::new(
                invocation.invocation_id.to_string(),
                PutInvocation,
                BackendFailure(err.into()),
            )
        })?;

        Ok(())
    }

    async fn get_invocation(&self, invocation_id: &str) -> Result<Request, StateError> {
        let output: GetItemOutput = self
            .get_item(&[(INVOCATION_ID, invocation_id)])
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

        let invocation: WorkflowInvocation<Request> =
            serde_dynamo::from_item(item).map_err(|err| {
                StateError::new(
                    invocation_id.to_string(),
                    GetInvocation,
                    BadState(err.to_string()),
                )
            })?;

        Ok(invocation.request)
    }

    async fn put_task(&self, task: WorkflowTask) -> Result<(), StateError> {
        let item: HashMap<String, AttributeValue> =
            serde_dynamo::to_item(&task).map_err(|err| {
                StateError::new(task.task_id.clone(), PutTask, BadState(err.to_string()))
            })?;

        self.put_item(item).await.map_err(|err| {
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
            .get_item(&[(INVOCATION_ID, invocation_id), (TASK_ID, task_id)])
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

impl<Request: Serialize + DeserializeOwned + Clone + Send + Sync> DynamoDbStateStore<Request> {
    async fn get_item(
        &self,
        key_parts: &[(&str, &str)],
    ) -> Result<GetItemOutput, SdkError<GetItemError, HttpResponse>> {
        let key: HashMap<String, AttributeValue> = key_parts
            .iter()
            .map(|&(k, v)| (k.to_string(), AttributeValue::S(v.to_string())))
            .collect();

        self.dynamodb_client
            .get_item()
            .consistent_read(self.consistent_read)
            .set_key(Some(key))
            .send()
            .await
    }

    async fn put_item(
        &self,
        item: HashMap<String, AttributeValue>,
    ) -> Result<PutItemOutput, SdkError<PutItemError, HttpResponse>> {
        self.dynamodb_client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_invocation_put() {}
}
