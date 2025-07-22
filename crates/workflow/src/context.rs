use lambda_runtime::tracing;
use model::task::{TaskId, WorkflowTask, WorkflowTaskState};
use model::{Error, InvocationId, WorkflowError};
use serde::de::DeserializeOwned;
use service::Dispatcher;
use service::{Service, ServiceError, ServiceRequest};
use state::{StateError, StateStore};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

pub struct WorkflowContext<T: DeserializeOwned + Clone + InvocationId> {
    request: T,
    pub(crate) state_store: Arc<dyn StateStore<T>>,
    callback_queue_url: String,

    calls: Arc<Mutex<Vec<Call>>>,
}

struct Call {
    task_id: String,
    // Result or an error if it failed to parse
    request: Result<String, Error>,

    dispatcher: Arc<dyn Dispatcher>,
}

#[macro_export]
macro_rules! call {
    ($ctx:expr, $service:expr, $payload:expr) => {{
        let id: String = format!("{}{}{}", file!(), line!(), column!());

        $ctx.call($service, id, $payload)
    }};
}

impl<WorkflowRequest: DeserializeOwned + Clone + InvocationId + Send + serde::Serialize>
WorkflowContext<WorkflowRequest>
{
    pub fn new(
        request: WorkflowRequest,
        state_store: Arc<dyn StateStore<WorkflowRequest>>,
        callback_queue_url: String,
    ) -> Self {
        WorkflowContext {
            request,
            state_store,
            callback_queue_url,
            calls: Default::default(),
        }
    }

    pub fn request(&self) -> &WorkflowRequest {
        &self.request
    }

    /// Call an async service which won't return immediately.
    /// This will suspend execution until a response is received.
    pub fn call<
        Payload: serde::Serialize + TaskId + 'static,
        Response: DeserializeOwned + TaskId + 'static,
    >(
        &self,
        service: &impl Service<Payload, Response>,
        // A unique id for this particular call
        unique_id: String,
        payload: Payload,
    ) -> impl Future<Output=Result<Response, WorkflowError>> {
        let invocation_id: String = self.request.invocation_id().to_string();
        let task_id: String = unique_id + payload.task_id();

        tracing::info!(service = service.name(), task_id = task_id, "Service call");

        let request: ServiceRequest<Payload> = ServiceRequest::new(
            invocation_id.to_string(),
            task_id.clone(),
            self.callback_queue_url.clone(),
            payload,
        );
        // Convert the call payload to a string for storage
        let request: Result<String, Error> =
            serde_json::to_string(&request).map_err(|err| err.into());
        let dispatcher: Arc<dyn Dispatcher> = service.dispatcher().clone();

        tokio::task::block_in_place(|| {
            let mut calls: MutexGuard<Vec<Call>> = self.calls.blocking_lock();
            // Store the call within the context
            calls.push(Call {
                task_id: task_id.clone(),
                request,
                dispatcher,
            });
        });

        let state_store: Arc<dyn StateStore<WorkflowRequest>> = self.state_store.clone();

        // Return a future which will make all calls before suspending
        async move {
            let task_result: Result<WorkflowTask, StateError> = state_store
                .get_task(invocation_id.as_str(), task_id.as_str())
                .await;

            if task_result.is_ok() {
                if let WorkflowTaskState::Completed(payload) = task_result
                    .map_err(|err| WorkflowError::Error(err.into()))?
                    .state
                {
                    tracing::debug!("Task completed and result available");

                    // Try and convert the result into the expected value
                    return serde_json::from_value(payload.clone()).map_err(|err| {
                        WorkflowError::Error(ServiceError::BadResponse(err.to_string()).into())
                    });
                }
            }

            // Invoke all queued external tasks
            self.dispatch_calls().await?;

            // Once ALL tasks have been dispatched, the function can be suspended
            Err(WorkflowError::Suspended)
        }
    }

    async fn dispatch_calls(&self) -> Result<(), WorkflowError> {
        let invocation_id: String = self.request.invocation_id().to_string();

        let mut calls: MutexGuard<Vec<Call>> = self.calls.lock().await;
        for call in calls.iter() {
            let task_id: String = call.task_id.clone();
            let request: &Result<String, Error> = &call.request;

            let request: String = request
                .as_ref()
                .map_err(|err| {
                    WorkflowError::Error(
                        // TODO better to keep the original error
                        ServiceError::BadRequest(Error::from(err.to_string())).into(),
                    )
                })?
                .clone();

            let running_task: WorkflowTask = WorkflowTask {
                invocation_id: invocation_id.to_string(),
                task_id: task_id.to_string(),
                state: WorkflowTaskState::Started,
            };

            // Store the running task and dispatch the request
            self.state_store
                .put_task(running_task)
                .await
                .map_err(|err| WorkflowError::Error(err.into()))?;

            call.dispatcher.send_message(request.clone()).await?;
        }

        // Prevent other threads from making duplicate invocations
        calls.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::context::WorkflowContext;
    use async_trait::async_trait;
    use model::task::{TaskId, WorkflowTask, WorkflowTaskState};
    use model::{Error, InvocationId};
    use service::{Dispatcher, Service};
    use state::{StateError, StateStore};
    use state_in_memory::InMemoryStateStore;
    use std::sync::Arc;
    use test_utils::{setup_default_env, TestStr};

    struct TestDispatcher;

    #[async_trait]
    impl Dispatcher for TestDispatcher {
        async fn send_message(&self, _payload: String) -> Result<(), Error> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct TestService;

    impl Service<TestStr, TestStr> for TestService {
        fn name(&self) -> &'static str {
            "test service"
        }

        fn dispatcher(&self) -> Arc<dyn Dispatcher> {
            Arc::new(TestDispatcher {})
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn single_call_invoked() {
        setup_default_env();

        let state_store: Arc<InMemoryStateStore<TestStr>> = Arc::new(InMemoryStateStore::default());
        let unique_id: String = "example".to_string();
        let request: TestStr = TestStr("test 1".to_string());

        let ctx: WorkflowContext<TestStr> =
            WorkflowContext::new(request.clone(), state_store.clone(), "test".to_string());

        let test_service: TestService = TestService {};

        let response_fut = ctx.call(
            &test_service,
            unique_id.clone(),
            request.clone(),
        );

        let task_id: String = unique_id + request.task_id();

        let task: Result<WorkflowTask, StateError> = state_store
            .get_task(request.invocation_id(), task_id.as_str())
            .await;
        // Task should not exist before the future is awaited
        assert!(task.is_err());

        let _response = response_fut.await;

        let task: WorkflowTask = state_store
            .get_task(request.invocation_id(), task_id.as_str())
            .await
            .expect("The second time the task should exist");

        assert!(matches!(task.state, WorkflowTaskState::Started));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn multiple_calls_invoked_concurrently() {
        setup_default_env();

        let state_store: Arc<InMemoryStateStore<TestStr>> = Arc::new(InMemoryStateStore::default());
        let unique_id: String = "example".to_string();
        let request_one: TestStr = TestStr("test 1".to_string());
        let request_two: TestStr = TestStr("test 2".to_string());

        let ctx: WorkflowContext<TestStr> =
            WorkflowContext::new(request_one.clone(), state_store.clone(), "test".to_string());

        let test_service: TestService = TestService {};

        let response_fut_1 = ctx.call(
            &test_service,
            unique_id.clone(),
            request_one.clone(),
        );
        let response_fut_2 = ctx.call(
            &test_service,
            unique_id.clone(),
            request_two.clone(),
        );

        // Both futures joined concurrently
        futures::future::join_all(vec![response_fut_1, response_fut_2]).await;

        state_store
            .get_task(request_one.invocation_id(), &*(unique_id.clone() + request_one.task_id()))
            .await
            .expect("The first task should exist");
        state_store
            .get_task(request_one.invocation_id(), &*(unique_id + request_two.task_id()))
            .await
            .expect("The second task should also exist");
    }
}
