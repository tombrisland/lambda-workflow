## TODO

### Next

1. Update DynamoDB module
   2. Add an in-memory cache which is initialised on first call with a query on the partition key
      3. This is then used on successive reads
3. Look at changing the StateError struct to an enum
   4. Maybe a wrapper around it which contains the invocation_id + call_id? Could be useful elsewhere too
   5. Change the response to be Result<Option<>> - more messy but makes more sense given it's not an error case to be missing state. (Is it alwasy for request?) This will then mean we don't need the invocation and task context inside the error.
8. Add some save method for long running operations
9. Clear out the old invocations once they get to the end of the workflow
   1. This includes any old call data? Add an option for that... it might be advantageous in some situations to keep it but for in-memory and on-disk initially probably don't want to.
12. Add an implementation for redis
13. Can the StateStore just be provided as an instance that implements cloneable and deref to a StateStore? Then we could drop the Arc wrapper on most impls