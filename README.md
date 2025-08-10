# Lambda Workflow

## Features

lambda-workflow is a serverless workflow runtime. The project enables the writing of lambda functions which orchestrate any number of asynchronous services without running into function timeouts.

* Write workflows as code
* Execute workflows with a serverless technology
* Less expensive than serverless orchestrators (aaS)

### How it works
1. Make an asynchronous call using `WorkflowContext::call()` or the `call!` macro
2. The lambda *suspends* by returning `WorkflowError::Suspended`
3. A service re-invokes the lambda by callback, storing the result in a state store.
   1. All results are then replayed when the user code is invoked

### Why?

Serverless orchestrators like Step Functions already exist, which are great for offloading scaling concerns to cloud providers. However, to make use of asynchronous patterns like `WaitForTaskToken`, we have to use [standard workflows](https://docs.aws.amazon.com/step-functions/latest/dg/choosing-workflow-type.html) which come with significant [cost](https://aws.amazon.com/step-functions/pricing/) attached.

AWS Lambda is great for short-running tasks (*sub 15 minutes*), but the runtime limitation makes it unsuitable for orchestrating longer running tasks or decoupled services. This project aims to enable workflows of any duration to run in Lambda.


## Example

Define a workflow in Rust code and run it as a Lambda. See the implementations in [examples](./examples).

```rust
// Define a workflow with a defined request and response type
async fn workflow_example(
   ctx: WorkflowContext<RequestExample>,
) -> Result<ResponseExample, WorkflowError> {
   let request: &RequestExample = ctx.request();

   let id: String = request.id.clone();
   let item_id: String = request.item_id.clone();

   tracing::info!("Making a request in an example workflow: {:?}", request);

   let service_request: ExampleServiceRequest = ExampleServiceRequest(item_id.clone());

   // Make requests to asynchronous services
   // The function will suspend until the call returns
   let result: ExampleServiceResponse = call!(ctx, &ExampleService {}, service_request).await?;

   Ok(ResponseExample {
      id,
      item_id,
      payload: result.0,
   })
}
```