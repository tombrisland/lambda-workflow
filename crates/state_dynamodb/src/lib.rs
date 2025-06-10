use crate::idempotency_record::{IDEMPOTENCY_KEY, RESPONSE_DATA};
use async_trait::async_trait;
use aws_sdk_dynamodb::operation::get_item::GetItemOutput;
use aws_sdk_dynamodb::types::AttributeValue;
use model::CallState;
use serde::de::DeserializeOwned;
use state::{BadStateReason, StateError, StateStore};
use std::collections::HashMap;
use std::marker::PhantomData;

mod idempotency_record;

struct DynamoDbStateStore<T> {
    dynamodb_client: aws_sdk_dynamodb::Client,
    // Remember expected request type
    phantom_data: PhantomData<T>,
}

#[async_trait]
impl<T: DeserializeOwned + Clone + Send + Sync> StateStore<T> for DynamoDbStateStore<T> {
    async fn put_invocation(&self, invocation_id: &str, request: T) -> Result<(), StateError> {
        Ok(())
    }

    async fn get_invocation(&self, invocation_id: &str) -> Result<T, StateError> {
        let output: GetItemOutput = self.get(invocation_id.to_string()).await?;

        Self::<T>::get_payload(output)
    }

    async fn put_call(
        &self,
        invocation_id: &str,
        call_id: &str,
        state: CallState,
    ) -> Result<(), StateError> {
        todo!()
    }

    async fn get_call(&self, invocation_id: &str, call_id: &str) -> Result<CallState, StateError> {
        todo!()
    }
}

// Lightweight helpers for use with DynamoDB
impl<T: DeserializeOwned + Clone> DynamoDbStateStore<T> {
    // Pull out an item by key
    async fn get(&self, key: String) -> Result<GetItemOutput, StateError> {
        self.dynamodb_client
            .get_item()
            .key(IDEMPOTENCY_KEY, AttributeValue::S(key))
            .send()
            .await
            .map_err(|err| StateError::BackendFailure(err.into()))
    }

    fn get_payload<P: DeserializeOwned>(key: &str, output: GetItemOutput) -> Result<P, StateError> {
        let item_map: HashMap<String, AttributeValue> = output
            .item
            // Error if the item doesn't exist
            .ok_or_else(|| StateError::MissingState(key.to_string()))?;

        // Extract the payload from the DynamoDB item
        let payload: &String = item_map
            .get(RESPONSE_DATA)
            // Error if the item is malformed in any way
            .ok_or_else(|| StateError::BadState {
                key: key.to_string(),
                reason: BadStateReason::MissingPayload,
            })?
            .as_s()
            .map_err(|_| StateError::BadState {
                key: key.to_string(),
                reason: BadStateReason::BadPayload(None),
            })?;

        // Serialise into the required JSON value
        serde_json::from_str(&payload).map_err(|err| StateError::BadState {
            key: key.to_string(),
            reason: BadStateReason::BadPayload(Some(err.to_string())),
        })
    }

    async fn put(&self, key: String) -> Result<GetItemOutput, StateError> {
        self.dynamodb_client
            .get_item()
            .key(IDEMPOTENCY_KEY, AttributeValue::S(key))
            .send()
            .await
            .map_err(|err| StateError::BackendFailure(err.into()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_engine_initialises_request() {}
}
