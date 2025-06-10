/// Table representing a workflow request or call.
/// This matches the `IdempotencyRecord` used by other lambda-powertools libraries.
/// 
/// https://docs.powertools.aws.dev/lambda/typescript/latest/utilities/idempotency/
struct IdempotencyRecord {
    idempotency_key: String,
    status: Status,
    expiry_timestamp: u64,
    in_progress_expiry_timestamp: u64,
    response_data: serde_json::Value,
}

pub(crate) const IDEMPOTENCY_KEY: &str = "idempotency_key";
pub(crate) const RESPONSE_DATA: &str = "response_data";

enum Status {
    InProgress,
    Complete,
    Expired,
}
