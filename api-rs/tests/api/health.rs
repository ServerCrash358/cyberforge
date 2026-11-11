//! `GET /health` and the error shape of unknown routes and methods.

use crate::common::TestApp;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn health_is_ok_true() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/health").await,
        (StatusCode::OK, json!({"ok": true}))
    );
}

#[tokio::test]
async fn unknown_route_is_404_with_an_error() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/nope").await,
        (StatusCode::NOT_FOUND, json!({"error": "not found"}))
    );
}

#[tokio::test]
async fn wrong_method_is_405_with_an_error() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/reports").await,
        (
            StatusCode::METHOD_NOT_ALLOWED,
            json!({"error": "method not allowed"})
        )
    );
}
