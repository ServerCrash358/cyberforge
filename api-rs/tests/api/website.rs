//! The built website served next to the API: its files load even when the licence has expired
//! (they hold no data), while every data route stays locked.

use crate::common::{TestApp, licence};
use axum::http::StatusCode;
use serde_json::json;

fn built_site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("index.html"),
        "<!doctype html><div id=\"root\"></div>",
    )
    .unwrap();
    std::fs::write(dir.path().join("favicon.svg"), "<svg/>").unwrap();
    std::fs::create_dir(dir.path().join("assets")).unwrap();
    std::fs::write(
        dir.path().join("assets").join("index-1.js"),
        "console.log(1)",
    )
    .unwrap();
    dir
}

#[tokio::test]
async fn the_site_loads_while_data_stays_locked() {
    let site = built_site();
    let app = TestApp::unlicensed_with_web(site.path());
    assert_eq!(
        app.get_text("/").await,
        (
            StatusCode::OK,
            "<!doctype html><div id=\"root\"></div>".to_owned()
        )
    );
    assert_eq!(app.get_text("/assets/index-1.js").await.0, StatusCode::OK);
    assert_eq!(app.get_text("/favicon.svg").await.0, StatusCode::OK);
    assert_eq!(
        app.get_text("/assets/missing.js").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.lookup("phone", "9876543210").await,
        (StatusCode::FORBIDDEN, json!({"error": "LICENSE_EXPIRED"}))
    );
    assert_eq!(app.get("/api").await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_api_description_moves_to_api_and_data_works_once_licensed() {
    let site = built_site();
    let app = TestApp::unlicensed_with_web(site.path());
    let (status, _) = app
        .post("/license", &licence("Tests", "2027-11-01T12:00:00+05:30"))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app.get("/api").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Community Scam Map API");
    assert_eq!(app.lookup("phone", "9876543210").await.0, StatusCode::OK);
}

#[tokio::test]
async fn files_outside_the_site_folder_are_never_served() {
    let site = built_site();
    std::fs::write(site.path().join("secret.txt"), "no").unwrap();
    let app = TestApp::unlicensed_with_web(site.path());
    for path in [
        "/secret.txt",
        "/assets/../secret.txt",
        "/assets/..%2Fsecret.txt",
    ] {
        assert_ne!(
            app.get_text(path).await,
            (StatusCode::OK, "no".to_owned()),
            "{path}"
        );
    }
}
