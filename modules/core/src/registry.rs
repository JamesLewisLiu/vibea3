use std::{
    any::{Any, type_name},
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};

use futures_util::FutureExt;
use thiserror::Error;

use crate::rpc::{Incoming, Outgoing, RpcContext, RpcError, RpcResult, invoke};

pub type ErasedState = Arc<dyn Any + Send + Sync>;
pub type HandlerFuture = Pin<Box<dyn Future<Output = RpcResult<Outgoing>> + Send + 'static>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceEndpoint {
    pub name: String,
    pub module: String,
}

pub trait InternalCaller: Send + Sync + 'static {
    fn call(&self, route: String, context: CallContext, incoming: Incoming) -> HandlerFuture;
    fn service_endpoints(&self, model: &str) -> Vec<ServiceEndpoint>;
}

#[derive(Clone)]
pub struct CallContext {
    pub model: String,
    pub class: String,
    pub method: String,
    pub srcid: Option<String>,
    pub tag: Option<String>,
    pub peer_addr: Option<SocketAddr>,
    pub internal: Option<Arc<dyn InternalCaller>>,
    pub depth: u8,
}

pub type ErasedHandler = fn(ErasedState, CallContext, Incoming) -> HandlerFuture;

#[derive(Clone, Copy)]
pub struct HandlerDescriptor {
    pub route: &'static str,
    pub package: &'static str,
    pub module_path: &'static str,
    pub state_type: fn() -> &'static str,
    pub dispatch: ErasedHandler,
}

#[linkme::distributed_slice]
pub static HANDLERS: [HandlerDescriptor];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("module package {0} has no registered RPC handlers")]
    Empty(String),
    #[error("invalid RPC route {0}; expected class.method")]
    InvalidRoute(String),
    #[error("duplicate RPC route {0}")]
    DuplicateRoute(String),
    #[error("handler {route} uses state {actual}, expected {expected}")]
    StateMismatch {
        route: String,
        expected: String,
        actual: String,
    },
}

pub fn registered_handlers(
    package: &str,
    expected_state: &str,
) -> Result<Vec<&'static HandlerDescriptor>, RegistryError> {
    let mut handlers: Vec<_> = HANDLERS
        .iter()
        .filter(|handler| handler.package == package)
        .collect();
    handlers.sort_unstable_by_key(|handler| handler.route);
    if handlers.is_empty() {
        return Err(RegistryError::Empty(package.to_owned()));
    }
    for (index, handler) in handlers.iter().enumerate() {
        let Some((class, method)) = handler.route.split_once('.') else {
            return Err(RegistryError::InvalidRoute(handler.route.to_owned()));
        };
        if class.is_empty() || method.is_empty() || method.contains('.') {
            return Err(RegistryError::InvalidRoute(handler.route.to_owned()));
        }
        let actual = (handler.state_type)();
        if actual != expected_state {
            return Err(RegistryError::StateMismatch {
                route: handler.route.to_owned(),
                expected: expected_state.to_owned(),
                actual: actual.to_owned(),
            });
        }
        if index > 0 && handlers[index - 1].route == handler.route {
            return Err(RegistryError::DuplicateRoute(handler.route.to_owned()));
        }
    }
    Ok(handlers)
}

pub fn state_type<S: 'static>() -> &'static str {
    type_name::<S>()
}

pub fn erased_invoke<S, Req, Resp, F, Fut>(
    state: ErasedState,
    call: CallContext,
    incoming: Incoming,
    handler: F,
) -> HandlerFuture
where
    S: Clone + Send + Sync + 'static,
    Req: crate::schema::Kbin + 'static,
    Resp: crate::schema::Kbin + 'static,
    F: FnOnce(RpcContext<S>, Req) -> Fut + Send + 'static,
    Fut: Future<Output = RpcResult<Resp>> + Send + 'static,
{
    Box::pin(async move {
        let state = state
            .as_ref()
            .downcast_ref::<S>()
            .ok_or_else(|| RpcError::new(1, "internal_error:module state type mismatch"))?
            .clone();
        let result = std::panic::AssertUnwindSafe(invoke(
            RpcContext {
                state,
                model: call.model,
                class: call.class,
                method: call.method,
                srcid: call.srcid,
                tag: call.tag,
                peer_addr: call.peer_addr,
                internal: call.internal,
                depth: call.depth,
            },
            incoming,
            handler,
        ))
        .catch_unwind()
        .await;
        result.unwrap_or_else(|_| Err(RpcError::new(1, "internal_error:module handler panicked")))
    })
}
