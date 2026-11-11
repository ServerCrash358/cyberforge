//! `GET /map?pincode&days=7&radius_km=2`: one cluster per area in radius and window.
//! Area points from `shared/data/areas.json` only; counted reports only.

use crate::body::{Fields, query_object};
use crate::core::areas;
use crate::core::isotime::to_utc_text;
use crate::core::mapview::{self, AreaCount, Severity};
use crate::errors::ApiError;
use crate::state::AppState;
use crate::store;
use axum::Json;
use axum::extract::{RawQuery, State};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use time::Duration;

static PINCODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{6}$").unwrap());

/// A `pincode` query parameter of six digits; `None`, with the problem noted, otherwise.
pub fn six_digit_pincode(f: &mut Fields<'_>) -> Option<String> {
    f.required("pincode", |s| PINCODE.is_match(s).then(|| s.to_owned()))
}

/// # Errors
/// A pincode that passed the pattern but is not one of the areas.
pub fn check_known(pincode: &str) -> Result<(), ApiError> {
    if areas::is_known(pincode) {
        Ok(())
    } else {
        Err(ApiError::bad_request("invalid pincode: not a known area"))
    }
}

#[derive(Serialize)]
pub struct Cluster {
    pincode: String,
    locality: String,
    city: String,
    count: usize,
    top_scam_type: String,
    severity: Severity,
    lat: f64,
    lng: f64,
}

#[derive(Serialize)]
pub struct MapOut {
    clusters: Vec<Cluster>,
}

pub async fn area_map(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Json<MapOut>, ApiError> {
    let params = query_object(query.as_deref());
    let mut f = Fields::new(&params);
    let pincode = six_digit_pincode(&mut f);
    let days = f.number_or("days", 7.0, |d| d > 0.0 && d <= 3650.0);
    let radius_km = f.number_or("radius_km", 2.0, |r| (0.0..=5000.0).contains(&r));
    f.finish()?;
    let pincode = pincode.expect("finish() refuses a missing pincode");
    check_known(&pincode)?;

    let now = state.clock.now();
    let nearby = mapview::areas_within(&pincode, radius_km);
    let since = to_utc_text(now - Duration::seconds_f64(days * 86_400.0));
    let counts = state
        .db
        .read(|conn| store::area_counts(conn, &nearby, &since, &to_utc_text(now)))?;
    let counts: HashMap<String, AreaCount> = counts
        .into_iter()
        .map(|(pin, (count, top_scam_type))| {
            (
                pin,
                AreaCount {
                    count,
                    top_scam_type,
                },
            )
        })
        .collect();
    let clusters = mapview::clusters(&counts)
        .into_iter()
        .map(|c| Cluster {
            pincode: c.area.pincode.clone(),
            locality: c.area.locality.clone(),
            city: c.area.city.clone(),
            count: c.count,
            top_scam_type: c.top_scam_type,
            severity: c.severity,
            lat: c.area.lat,
            lng: c.area.lng,
        })
        .collect();
    Ok(Json(MapOut { clusters }))
}
