//! Middleware run on every request: expired means 403 `LICENSE_EXPIRED`, except on `/health` and
//! `/license`, which always work, and the website's own files, which hold no data and must load so
//! the site can show "Licence expired".

use crate::errors::{ApiError, LICENSE_EXPIRED};
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const OPEN_PATHS: [&str; 4] = ["/health", "/license", "/", "/favicon.svg"];
const OPEN_PREFIX: &str = "/assets/";

pub async fn licence_gate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if OPEN_PATHS.contains(&path) || path.starts_with(OPEN_PREFIX) {
        return next.run(request).await;
    }
    match state.licence.is_valid(&state.db, state.clock.now()) {
        Ok(true) => next.run(request).await,
        Ok(false) => ApiError::new(StatusCode::FORBIDDEN, LICENSE_EXPIRED).into_response(),
        Err(e) => ApiError::from(e).into_response(),
    }
}
