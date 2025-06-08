// use crate::model::WorkflowRequest;
// use crate::workflow_engine::{WorkflowContext, WorkflowEngine};
// use lambda_runtime::Service;
// use std::future::Future;
// use std::task::{Context, Poll};
// 
// pub fn workflow_fn<Request : WorkflowRequest, Function>(engine: WorkflowEngine<Request>, f: Function) -> WorkflowFn<Request, Function> {
//     WorkflowFn { engine, f }
// }
// #[derive(Clone)]
// pub struct WorkflowFn<Request : WorkflowRequest, Function> {
//     engine: WorkflowEngine<Request>,
//     f: Function,
// }
// 
// impl<Function, Fut, Request, Response, Error> Service<Request> for WorkflowFn<Request, Function>
// where
//     Request: WorkflowRequest,
//     Function: FnMut(WorkflowContext<Request>) -> Fut,
//     Fut: Future<Output = Result<Response, Error>>,
// {
//     type Response = Response;
//     type Error = Error;
//     type Future = Fut;
// 
//     fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         Ok(()).into()
//     }
// 
//     fn call(&mut self, req: Request) -> Self::Future {
//         (self.f)(self.engine.accept(req).unwrap())
//     }
// }
