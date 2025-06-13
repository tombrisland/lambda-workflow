# In flight
1. Update DynamoDB module
   1. Use ConsistentRead
   2. Add an in-memory cachewhich is initialised on first call with a query on the partition key
      3. This is then used on successive reads
   3. Look at changing the StateError struct to an enum
      4. Maybe a wrapper around it which contains the invocation_id + call_id? Could be useful elsewhere too
   5. Put some tests in around #2

# What's next?

1. Flesh out the workflow_sqs so that it works in lambda
   2. Might need a test Lambda to do something else unless it can be fully mocked
5. Some clever way of storing what things a particular workflow is waiting for
   1. e.g. no point running the workflow through again if we're waiting on multiple things to happen
6. A persistent state store which writes to disk
   1. To be used in Lambda initially with a single concurrency
7. Why does the request have to be bounded with a static lifetime?
8. Add multiple helper methods to the context
   1. callMultiple or callAll() - for when you want to wait for multiple tasks but fire them off together
   2. Some save that can be performed on long running operations to avoid replaying them (.run() with a save feature like checkpointing code)
9. Clear out the old invocations once they get to the end of the workflow
   1. This includes any old call data? Add an option for that... it might be advantageous in some situations to keep it but for in-memory and on-disk initially probably don't want to.
10. Put in some trait aliases to reduce the huge trait bounds