//! `GET /alerts?pincode`: SPIKE and `NEW_TYPE` on counted reports only. None is `{"alerts": []}`.

use super::map::{check_known, six_digit_pincode};
use crate::body::{Fields, query_object};
use crate::core::alerts::{self as alert_rules, Alert};
use crate::core::isotime::to_utc_text;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::store;
use axum::Json;
use axum::extract::{RawQuery, State};
use serde::Serialize;

#[derive(Serialize)]
pub struct AlertsOut {
    alerts: Vec<Alert>,
}

pub async fn alerts(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Json<AlertsOut>, ApiError> {
    let params = query_object(query.as_deref());
    let mut f = Fields::new(&params);
    let pincode = six_digit_pincode(&mut f);
    f.finish()?;
    let pincode = pincode.expect("finish() refuses a missing pincode");
    check_known(&pincode)?;

    let now = state.clock.now();
    let since = to_utc_text(now - alert_rules::HISTORY);
    let events = state
        .db
        .read(|conn| store::area_events(conn, &pincode, &since))?;
    Ok(Json(AlertsOut {
        alerts: alert_rules::alerts(&pincode, &events, now),
    }))
}
