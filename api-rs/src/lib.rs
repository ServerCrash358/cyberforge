//! Community Scam Map API in Rust. `app()` builds the router so tests can call it without a socket;
//! `app_with_web()` also serves the built website.

pub mod activity;
mod body;
pub mod clock;
pub mod config;
pub mod core;
pub mod db;
pub mod errors;
pub mod license;
mod routes;
mod shared;
pub mod state;
pub mod store;

use std::path::Path;

/// The API alone.
pub fn app(state: state::AppState) -> axum::Router {
    routes::router(state, None)
}

/// The API and the website built in `web` (the `ui/dist` folder).
pub fn app_with_web(state: state::AppState, web: &Path) -> axum::Router {
    routes::router(state, Some(web))
}
