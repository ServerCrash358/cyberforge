//! Starts the API: reads settings, opens the database, binds the address, serves until Ctrl+C.
//! Like the Python API, it logs no requests: only startup and shutdown lines.
//!
//! `scam-map-api backup <file>` instead copies the database named by `SCAM_MAP_DB_PATH` to <file>,
//! safely while the server runs (the server's nightly backup).

use scam_map_api::clock::SystemClock;
use scam_map_api::config::Config;
use scam_map_api::core::identity::Keyring;
use scam_map_api::db::Database;
use scam_map_api::license::verify::load_public_key;
use scam_map_api::state::{AppState, Options};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [command, target] = args.as_slice()
        && command == "backup"
    {
        Database::open(&config.db_path)?.backup_to(std::path::Path::new(target))?;
        eprintln!("backed up to {target}");
        return Ok(());
    }
    if !args.is_empty() {
        return Err("usage: scam-map-api [backup <file>]".into());
    }
    let key = config
        .hmac_key
        .as_deref()
        .ok_or("SCAM_MAP_HMAC_KEY is not set; see docs/SETUP.md")?;
    let public_key = load_public_key(config.license_public_key.as_deref());
    if public_key.is_none() {
        eprintln!("no usable licence public key: every licence will be refused");
    }
    let options = Options {
        admin_token: config.admin_token.clone(),
        trust_proxy: config.trust_proxy,
    };
    let state = AppState::with_options(
        Database::open(&config.db_path)?,
        Arc::new(SystemClock),
        Keyring::new(key)?,
        public_key,
        options,
    )?;
    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    eprintln!("scam-map-api listening on {}", listener.local_addr()?);
    let app = match &config.web_dir {
        Some(dir) => scam_map_api::app_with_web(state, dir),
        None => scam_map_api::app(state),
    }
    .into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    eprintln!("scam-map-api stopped");
    Ok(())
}
