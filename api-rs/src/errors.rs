//! Error replies shaped `{"error": "..."}`, including exactly 403 `{"error": "LICENSE_EXPIRED"}`.
//! No identifiers, notes or query strings in any message.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub const LICENSE_EXPIRED: &str = "LICENSE_EXPIRED";

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

/// A database failure is a 500 with a fixed message: SQL errors may quote stored values.
impl From<rusqlite::Error> for ApiError {
    fn from(_: rusqlite::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

pub async fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not found")
}

pub async fn method_not_allowed() -> ApiError {
    ApiError::new(StatusCode::METHOD_NOT_ALLOWED, "method not allowed")
}
