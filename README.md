# Lambda Workflow

## Features

lambda-workflow is a serverless workflow runtime. The project enables the writing of lambda functions which orchestrate any number of asynchronous services without running into function timeouts.

* Write workflows as code
* Execute workflows with a serverless technology
* Less expensive than orchestrator cloud services

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

Below is an example around a simple bank transaction workflow.

```rust
async fn make_transaction(
   ctx: WorkflowContext<BankTransaction>,
   // Services can be passed in
   balance_service: BalanceService,
   fraud_service: FraudService,
   transaction_service: TransactionService,
) -> Result<TransactionResult, WorkflowError> {
   let transaction: &BankTransaction = ctx.request();

   // Both checks can be executed in parallel
   let (balance_result, fraud_result) = join!(
        // Retrieve the account balance
        call!(ctx, &balance_service, transaction.clone()),
        // Check the transaction for indicators of fraud
        call!(ctx, &fraud_service, transaction.clone())
    );

   // If we don't have enough money
   if balance_result?.balance < transaction.amount
           // Or the transaction is fraudulent
           || fraud_result?.is_likely_fraud {
      // Fail the transaction
      return Ok(TransactionResult::Failure);
   }

   // Otherwise we're good to make it
   call!(ctx, &transaction_service, transaction.clone()).await?;

   Ok(TransactionResult::Success)
}
```

### Deployment

A lambda-workflow should be deployed with a FIFO queue as the input.
This is important as the concurrency guarantees provided by `MessageGroupId` are integral to it's operation.
Multiple updates from a single invocation *cannot* be processed in parallel.