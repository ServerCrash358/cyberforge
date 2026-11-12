//! `POST /admin/reset`: deletes reports, cases, challenges and events. Keeps the areas, the
//! verified values and the licence. No login (PRD Appendix A rule 5), unless the server was started
//! with `SCAM_MAP_ADMIN_TOKEN` for the public internet; then the `x-admin-token` header must match.

use crate::errors::ApiError;
use crate::state::AppState;
use crate::store;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Compares digests rather than the strings, so the time taken says nothing about the token.
fn same_token(given: &[u8], expected: &str) -> bool {
    Sha256::digest(given) == Sha256::digest(expected.as_bytes())
}

pub async fn reset(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if let Some(expected) = &state.options.admin_token {
        let given = headers.get("x-admin-token").map(HeaderValue::as_bytes);
        if !given.is_some_and(|g| same_token(g, expected)) {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "missing or wrong admin token",
            ));
        }
    }
    state.db.write(|tx| store::reset(tx))?;
    Ok(Json(json!({"ok": true})))
}
