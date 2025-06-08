use std::fmt::Debug;
use crate::in_memory_state::InMemoryStateStore;
use crate::model::{
    CallResult, CallState, WorkflowError, WorkflowEvent, WorkflowId,
};
use crate::service::AsyncService;
use crate::state::StateStore;
use crate::{workflow_example, RequestExample};
use aws_lambda_events::sqs::{SqsEvent, SqsMessage};
use lambda_runtime::tracing::log::info;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::Deserialize;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct WorkflowEngine<T: DeserializeOwned + Clone + WorkflowId> {
    state_store: Arc<dyn StateStore<T>>,
}

impl<'a, Request: DeserializeOwned + Clone + WorkflowId> WorkflowEngine<Request> {
    pub fn new() -> WorkflowEngine<Request> {
        let state_store: InMemoryStateStore<Request> = InMemoryStateStore::default();
        
        WorkflowEngine {
            state_store: Arc::new(state_store),
        }
    }

    pub fn accept(&self, event: WorkflowEvent<Request>) -> Result<WorkflowContext<Request>, Error> {
        match event {
            WorkflowEvent::Request(request) => self.create_ctx(request),
            WorkflowEvent::Update(state) => self.update_ctx(state),
        }
    }

    fn create_ctx(&self, request: Request) -> Result<WorkflowContext<Request>, Error> {
        self.state_store
            .put_invocation(request.workflow_id(), request.clone())?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }

    fn update_ctx(&self, call_result: CallResult) -> Result<WorkflowContext<Request>, Error> {
        let workflow_id: &str = &call_result.workflow_id;
        let call_id: &str = &call_result.call_id.clone();

        let request: Request = self
            .state_store
            .get_invocation(workflow_id)
            .ok_or(format!("Failed to get workflow id {}", workflow_id).as_str())?;

        // Update the state with any calls
        self.state_store
            .put_call(call_id, CallState::Completed(call_result))?;

        Ok(WorkflowContext::new(request, self.state_store.clone()))
    }
}

pub struct WorkflowContext<T: DeserializeOwned + Clone + WorkflowId> {
    request: T,
    state_store: Arc<dyn StateStore<T>>,
}

impl<T: DeserializeOwned + Clone + WorkflowId> WorkflowContext<T> {
    pub fn new(request: T, state_store: Arc<dyn StateStore<T>>) -> Self {
        WorkflowContext {
            request,
            state_store,
        }
    }

    pub fn get_request(&self) -> &T {
        &self.request
    }

    pub async fn call(
        &self,
        service: impl AsyncService<String>,
        call_id: &str,
    ) -> Result<String, WorkflowError> {
        // Check if the result is already available in state
        if let Some(state) = self.state_store.get_call(call_id) {
            return match state {
                // Suspend if it's not available
                CallState::Running => Err(WorkflowError::Suspended),
                // Return the completed result
                CallState::Completed(result) => Ok(result.value.clone()),
            };
        }

        // Set the state running and then call the service
        self.state_store.put_call(call_id, CallState::Running)?;
        service.call(call_id.to_string()).await?;

        Err(WorkflowError::Suspended)
    }
}

pub(crate) async fn workflow_fn(
    engine: &WorkflowEngine<RequestExample>,
    event: LambdaEvent<SqsEvent>,
) -> Result<(), Error> {
    info!("Starting workflow event handler");

    info!("Finishing workflow event handler");

    Ok(())
}

async fn run<'a, Function, Request, Response, Fut>(f: Function) -> Result<(), Error>
where
    Request: Debug + Clone + DeserializeOwned + WorkflowId,
    Function: FnMut(WorkflowContext<Request>) -> Fut,
    Fut: Future<Output = Result<Response, WorkflowError>>,
{
    let engine: WorkflowEngine<Request> = WorkflowEngine::new();
    let engine_ref: &WorkflowEngine<Request> = &engine;

    let fn_ref: &Function = &f;

    let handler = move |event: LambdaEvent<SqsEvent>| async move {
        let records: Vec<SqsMessage> = event.payload.records;

        // Iterate the events from the SQS queue
        for record in records {
            let body: String = record.body.unwrap();
            
            let body = body.clone();
            let workflow_event: WorkflowEvent<Request> = serde_json::from_str(body.deref()).unwrap();

            info!(
                "Handling {:?} event for workflow_id {}",
                workflow_event,
                workflow_event.workflow_id()
            );

            let ctx: WorkflowContext<Request> = engine_ref.accept(workflow_event).unwrap();
            
            // For some reason it's requiring that Request have static lt
            // Function can't be passed in

            let x = fn_ref(ctx).await;
        }

        let x: Result<(), Error> = Ok(());
        x
    };

    lambda_runtime::run(service_fn(handler)).await
}
