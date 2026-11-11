//! Community Scam Map API in Rust. `app()` builds the router so tests can call it without a socket.

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

pub use routes::router as app;
