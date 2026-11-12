//! The PRD routes (Appendix A), the website helpers under `/app`, and the website itself.

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
use axum::routing::{get, get_service, post};
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};

/// Every route the API serves, all behind the licence gate (which lets `/health`, `/license` and
/// the website's own files through even when expired).
///
/// With `web`, the built website (`ui/dist`) is served at `/`, `/assets/*` and `/favicon.svg`;
/// without it, `/` describes the API like `/api` does.
pub fn router(state: AppState, web: Option<&Path>) -> Router {
    let site = match web {
        Some(dir) => Router::new()
            .route_service("/", get_service(ServeFile::new(dir.join("index.html"))))
            .route_service(
                "/favicon.svg",
                get_service(ServeFile::new(dir.join("favicon.svg"))),
            )
            .nest_service("/assets", ServeDir::new(dir.join("assets"))),
        None => Router::new().route("/", get(app::root)),
    };
    Router::new()
        .merge(site)
        .route("/api", get(app::root))
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
