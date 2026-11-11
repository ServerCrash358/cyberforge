//! Middleware run on every request: expired means 403 `LICENSE_EXPIRED`, except on `/health` and
//! `/license`, which always work.

use crate::errors::{ApiError, LICENSE_EXPIRED};
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const OPEN_PATHS: [&str; 2] = ["/health", "/license"];

pub async fn licence_gate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if OPEN_PATHS.contains(&request.uri().path()) {
        return next.run(request).await;
    }
    match state.licence.is_valid(&state.db, state.clock.now()) {
        Ok(true) => next.run(request).await,
        Ok(false) => ApiError::new(StatusCode::FORBIDDEN, LICENSE_EXPIRED).into_response(),
        Err(e) => ApiError::from(e).into_response(),
    }
}
