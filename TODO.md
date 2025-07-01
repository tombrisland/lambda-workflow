# In flight
1. Enable output via SQS but add indirection

# What's next?

1. Update DynamoDB module
   2. Add an in-memory cache which is initialised on first call with a query on the partition key
      3. This is then used on successive reads
3. Look at changing the StateError struct to an enum
   4. Maybe a wrapper around it which contains the invocation_id + call_id? Could be useful elsewhere too
6. A persistent state store which writes to disk
   1. To be used in Lambda initially with a single concurrency
8. Add multiple helper methods to the context
   1. callMultiple or callAll() - for when you want to wait for multiple tasks but fire them off together
   2. Some save that can be performed on long running operations to avoid replaying them (.run() with a save feature like checkpointing code)
9. Clear out the old invocations once they get to the end of the workflow
   1. This includes any old call data? Add an option for that... it might be advantageous in some situations to keep it but for in-memory and on-disk initially probably don't want to.
11. Put in an integration test using localstack
12. Add an implementation for redis
13. Can the StateStore just be provided as an instance that implements cloneable and deref to a StateStore? Then we could drop the Arc wrapper on most impls