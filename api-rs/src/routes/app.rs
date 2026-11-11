//! Helpers for the website, not judged: `GET /app/areas` for the area picker, `GET /app/activity`
//! for the live panel, and `GET /` describing the API. All behind the licence like the rest.

use crate::activity::{self, Feed};
use crate::errors::ApiError;
use crate::shared;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

const ROUTES: [&str; 9] = [
    "GET /health",
    "POST /reports",
    "GET /lookup?type=&value=",
    "GET /map?pincode=&days=7&radius_km=2",
    "GET /alerts?pincode=",
    "POST /admin/reset",
    "POST /challenges",
    "GET /license",
    "POST /license",
];

/// A short description, so opening the bare address is not an error page.
pub async fn root() -> Json<Value> {
    Json(json!({"name": "Community Scam Map API", "ok": true, "routes": ROUTES}))
}

pub async fn areas() -> Json<Value> {
    Json(json!({"areas": *shared::AREAS_RAW}))
}

/// Newest events first (masked, grouped, never a reporter field) and every alert live now.
pub async fn activity_feed(State(state): State<AppState>) -> Result<Json<Feed>, ApiError> {
    let now = state.clock.now();
    Ok(Json(state.db.read(|conn| activity::feed(conn, now))?))
}
