//! `GET /health`: always 200 `{"ok": true}`, even when the licence has expired.

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Health {
    ok: bool,
}

pub async fn health() -> Json<Health> {
    Json(Health { ok: true })
}
