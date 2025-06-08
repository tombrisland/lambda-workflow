# What's next?

1. Flesh out the workflow_sqs so that it works in lambda
   2. Might need a test Lambda to do something else unless it can be fully mocked
2. Create a DynamoDB module for the state store
   1. It should load all the items for the workflow in and use transactions
   2. Consider using the same table model as lambda-powertools idempotency store
4. Some tests around the engine and workflow
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