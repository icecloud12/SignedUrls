//! Characterization tests for bucket creation: `/project/create`, `/v1/bucket`, `/v2/bucket`.
//! They record current behavior, not necessarily the desired one.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use mongodb::bson::{doc, Document};
use serde_json::json;

/// All three routes point at the same handler.
const ROUTES: [&str; 3] = ["/project/create", "/v1/bucket", "/v2/bucket"];

#[tokio::test]
async fn create_bucket_returns_keys_on_every_route() {
    let app = TestApp::new().await;

    for (i, route) in ROUTES.iter().enumerate() {
        let name = format!("bucket-{i}");
        let res = app.post_json(route, json!({ "name": name })).await;

        assert_eq!(
            res.status,
            StatusCode::CREATED,
            "route {route}: {}",
            res.text()
        );
        let body = res.json();
        assert_eq!(body["name"], name);
        for field in ["_id", "public_key", "secret_key", "secret_key_hint"] {
            assert!(
                body[field].is_string(),
                "route {route}: missing {field} in {body}"
            );
        }
        // the hint is "***" plus the last 3 characters of the secret
        let secret = body["secret_key"].as_str().unwrap();
        assert_eq!(
            body["secret_key_hint"],
            format!("***{}", &secret[secret.len() - 3..])
        );
    }
}

#[tokio::test]
async fn create_bucket_stores_hashed_secret_and_creates_directory() {
    let app = TestApp::new().await;

    let body = app
        .post_json("/v2/bucket", json!({ "name": "stored" }))
        .await
        .json();
    let id = body["_id"].as_str().unwrap();
    let secret = body["secret_key"].as_str().unwrap();

    // buckets live in the "project" collection
    let stored = app
        .db
        .collection::<Document>("project")
        .find_one(doc! { "name": "stored" }, None)
        .await
        .unwrap()
        .expect("bucket document was not stored");
    let hashed = stored.get_str("hashed_secret_key").unwrap();
    assert!(
        hashed.starts_with("$argon2"),
        "expected an argon2 hash, got {hashed}"
    );
    assert_ne!(hashed, secret, "secret key must not be stored in plaintext");
    assert!(
        !stored.contains_key("api_key"),
        "new buckets have no v1 api_key"
    );

    assert!(
        app.data_dir.path().join(id).is_dir(),
        "bucket directory was not created"
    );
}

#[tokio::test]
async fn create_bucket_with_existing_name_is_a_conflict() {
    let app = TestApp::new().await;
    app.post_json("/v2/bucket", json!({ "name": "dup" })).await;

    let res = app.post_json("/v2/bucket", json!({ "name": "dup" })).await;

    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(
        res.json(),
        json!({ "code": "BUCKET_ALREADY_EXISTS", "message": "Bucket already exists" })
    );
}

#[tokio::test]
async fn create_bucket_without_name_is_unprocessable() {
    let app = TestApp::new().await;

    let res = app.post_json("/v2/bucket", json!({})).await;

    // axum's Json extractor rejects it before the handler runs
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
}
