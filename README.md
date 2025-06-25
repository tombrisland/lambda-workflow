# Lambda Workflow

`lambda-workflow` is a serverless workflow engine.

The project relies on using an external state store in conjunction with SQS messages to invoke asynchronous services, suspend operation, then resume upon receiving a callback.

* DynamoDB backend for state storage
* Write workflows as Rust code
* Call asynchronous services via SQS callbacks

# Example

Define a workflow in Rust code and run it as a Lambda. See the implementations in [examples](./examples).

```rust
// Define a workflow with a defined request and response type
async fn workflow_example(
    ctx: WorkflowContext<RequestExample>,
) -> Result<ResponseExample, WorkflowError> {
    let request: &RequestExample = ctx.request();
  
    let service_request: ExampleServiceRequest = ExampleServiceRequest(request.item_id.clone());
    // Make asynchronous requests to external services
    // The workflow will suspend until the request returns
    let result: ExampleServiceResponse = ctx.call(&ExampleService {}, service_request).await?;

    // The response from the workflow is delivered to an output queue
    Ok(ResponseExample {
        id: request.id.clone(),
        item_id: request.item_id.clone(),
        payload: result.0,
    })
}
```