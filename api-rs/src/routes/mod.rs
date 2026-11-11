//! The PRD routes (Appendix A) and the website helpers under `/app`.

mod admin;
mod alerts;
mod app;
mod challenges;
mod health;
mod license;
mod lookup;
mod map;
mod reports;

use crate::errors;
use crate::license::gate::licence_gate;
use crate::state::AppState;
use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};

/// Every route the API serves, all behind the licence gate (which lets `/health` and `/license`
/// through even when expired).
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(app::root))
        .route("/health", get(health::health))
        .route("/reports", post(reports::create))
        .route("/lookup", get(lookup::lookup))
        .route("/map", get(map::area_map))
        .route("/alerts", get(alerts::alerts))
        .route("/admin/reset", post(admin::reset))
        .route("/challenges", post(challenges::create))
        .route("/license", get(license::show).post(license::install))
        .route("/app/areas", get(app::areas))
        .route("/app/activity", get(app::activity_feed))
        .method_not_allowed_fallback(errors::method_not_allowed)
        .fallback(errors::not_found)
        .layer(from_fn_with_state(state.clone(), licence_gate))
        .with_state(state)
}
