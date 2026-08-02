use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;

use super::ModuleManager;

struct AdminState<H: ?Sized + Send + Sync + 'static> {
    manager: ModuleManager<H>,
    token: Arc<str>,
}

impl<H: ?Sized + Send + Sync + 'static> Clone for AdminState<H> {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
            token: self.token.clone(),
        }
    }
}

#[derive(Serialize)]
struct AdminResult<T: Serialize> {
    ok: bool,
    result: T,
}

pub(super) fn router<H: ?Sized + Send + Sync + 'static>(
    manager: ModuleManager<H>,
    token: Arc<str>,
) -> Router {
    let state = AdminState { manager, token };
    Router::new()
        .route("/_vibea3/admin/v1/modules", get(list::<H>))
        .route("/_vibea3/admin/v1/modules/rescan", post(rescan::<H>))
        .route(
            "/_vibea3/admin/v1/modules/{module_id}/reload",
            post(reload::<H>),
        )
        .route(
            "/_vibea3/admin/v1/modules/{module_id}/unload",
            post(unload::<H>),
        )
        .with_state(state)
}

async fn list<H: ?Sized + Send + Sync + 'static>(
    State(state): State<AdminState<H>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, &state.token) {
        return denied();
    }
    json(
        StatusCode::OK,
        AdminResult {
            ok: true,
            result: state.manager.statuses(),
        },
    )
}

async fn rescan<H: ?Sized + Send + Sync + 'static>(
    State(state): State<AdminState<H>>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, &state.token) {
        return denied();
    }
    operation(state.manager.rescan().await)
}

async fn reload<H: ?Sized + Send + Sync + 'static>(
    State(state): State<AdminState<H>>,
    Path(module_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, &state.token) {
        return denied();
    }
    operation(state.manager.reload(&module_id).await)
}

async fn unload<H: ?Sized + Send + Sync + 'static>(
    State(state): State<AdminState<H>>,
    Path(module_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, &state.token) {
        return denied();
    }
    operation(state.manager.unload(&module_id).await)
}

fn operation(result: Result<(), super::ModuleError>) -> Response {
    match result {
        Ok(()) => json(
            StatusCode::OK,
            AdminResult {
                ok: true,
                result: "ok",
            },
        ),
        Err(error) => json(
            StatusCode::CONFLICT,
            AdminResult {
                ok: false,
                result: error.to_string(),
            },
        ),
    }
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(candidate) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return false;
    };
    constant_time_eq(candidate.as_bytes(), token.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        difference |= usize::from(
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

fn denied() -> Response {
    json(
        StatusCode::UNAUTHORIZED,
        AdminResult {
            ok: false,
            result: "unauthorized",
        },
    )
}

fn json<T: Serialize>(status: StatusCode, value: T) -> Response {
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
