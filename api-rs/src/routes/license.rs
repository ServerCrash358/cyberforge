//! `GET /license` and `POST /license`: show the licence; verify and hot-swap a new one without a
//! restart. Both work when expired.

use crate::body::{Fields, json_object};
use crate::errors::ApiError;
use crate::license::state::InstallError;
use crate::license::verify::LicenceDoc;
use crate::state::AppState;
use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;

#[derive(Serialize)]
pub struct LicenseOut {
    licensee: Option<String>,
    expires_at: Option<String>,
    valid: bool,
}

pub async fn show(State(state): State<AppState>) -> Result<Json<LicenseOut>, ApiError> {
    let valid = state.licence.is_valid(&state.db, state.clock.now())?;
    Ok(Json(match state.licence.licence() {
        Some(l) => LicenseOut {
            licensee: Some(l.doc.licensee),
            expires_at: Some(l.doc.expires_at),
            valid,
        },
        None => LicenseOut {
            licensee: None,
            expires_at: None,
            valid: false,
        },
    }))
}

pub async fn install(
    State(state): State<AppState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<LicenseOut>, ApiError> {
    let map = json_object(&headers, &bytes)?;
    let mut f = Fields::new(&map);
    let doc = LicenceDoc {
        licensee: f.required_str("licensee", 1),
        expires_at: f.required_str("expires_at", 1),
        signature: f.required_str("signature", 1),
    };
    f.finish()?;
    let licence = state
        .licence
        .install(&state.db, &doc, state.clock.now())
        .map_err(|e| match e {
            InstallError::Invalid(why) => ApiError::bad_request(why.0),
            InstallError::Database(db) => ApiError::from(db),
        })?;
    let valid = state.licence.is_valid(&state.db, state.clock.now())?;
    Ok(Json(LicenseOut {
        licensee: Some(licence.doc.licensee),
        expires_at: Some(licence.doc.expires_at),
        valid,
    }))
}
