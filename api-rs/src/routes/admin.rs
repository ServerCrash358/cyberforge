//! `POST /admin/reset`: deletes reports, cases, challenges and events. Keeps the areas, the
//! verified values and the licence. No login (PRD Appendix A rule 5).

use crate::errors::ApiError;
use crate::state::AppState;
use crate::store;
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

pub async fn reset(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    state.db.write(|tx| store::reset(tx))?;
    Ok(Json(json!({"ok": true})))
}
