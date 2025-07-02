# Lambda Workflow

## Features

*lambda-workflow* is a serverless workflow runtime. The project enables the writing of lambda functions which orchestrate any number of asynchronous services without running into function timeouts.

* Write workflows as code
* Execute workflows with a serverless technology
* Avoid expensive serverless orchestrators (*aaS*)

### How it works
1. Make an asynchronous call using `WorkflowContext::call()`
2. The lambda *suspends* by returning `WorkflowError::Suspended`
3. A service re-invokes the lambda by callback, storing the result in a state store.
   1. All results are then replayed when the user code is invoked

### Why?

Lambda and other serverless function runtimes are excellent for offloading scaling concerns to CSPs at the cost of a premium. With AWS Lambda, you are limited to a maximum 15-minute runtime. This makes it unsuitable for orchestrating longer running tasks or decoupled services.

Of course, serverless orchestrators exist, Step Functions for example. However, to make use of asynchronous patterns like `WaitForTaskToken`, we are forced to use [standard workflows](https://docs.aws.amazon.com/step-functions/latest/dg/choosing-workflow-type.html) which have significant [cost](https://aws.amazon.com/step-functions/pricing/) attached.

## Example

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