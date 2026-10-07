//! Characterization tests for creating upload URLs:
//! `/request/create/upload`, `/v1/request/upload` (v1) and `/v2/request/upload` (v2).

mod common;

use axum::http::StatusCode;
use common::{TestApp, URL_BASE};
use mongodb::bson::{doc, oid::ObjectId};
use serde_json::{json, Value};

const V1_ROUTES: [&str; 2] = ["/request/create/upload", "/v1/request/upload"];

// ---------- v1 ----------

#[tokio::test]
async fn v1_returns_path_style_url_wrapped_in_data() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "legacy-key").await;

    for route in V1_ROUTES {
        let res = app.post_json(route, json!({ "api_key": "legacy-key" })).await;

        assert_eq!(res.status, StatusCode::CREATED, "route {route}: {}", res.text());
        let data = &res.json()["data"];
        let request_id = data["request_id"].as_str().unwrap();
        let url = data["url"].as_str().unwrap();
        let prefix = format!("{URL_BASE}/id/{request_id}/permission/upload_v1/created/");
        assert!(url.starts_with(&prefix), "route {route}: {url}");
        assert!(url.contains("/expiration/") && url.contains("/nonce/") && url.contains("/signature/"));
    }
}

#[tokio::test]
async fn v1_stores_request_with_defaults() {
    let app = TestApp::new().await;
    let project_id = app.insert_legacy_project("legacy", "legacy-key").await;

    let res = app
        .post_json("/v1/request/upload", json!({ "api_key": "legacy-key", "target": "images" }))
        .await;
    let request_id = res.json()["data"]["request_id"].as_str().unwrap().to_string();

    let stored = app
        .find_one("request", doc! { "_id": ObjectId::parse_str(&request_id).unwrap() })
        .await
        .unwrap();
    assert_eq!(stored.get_object_id("project_id").unwrap().to_hex(), project_id);
    assert_eq!(stored.get_str("permission").unwrap(), "upload_v1");
    assert_eq!(stored.get_str("target").unwrap(), "images");
    let duration = stored.get_i64("expiration_date").unwrap() - stored.get_i64("date_created").unwrap();
    assert_eq!(duration, 3600, "uses DEFAULT_DURATION_AS_SECONDS when duration is not given");
    let options = stored.get_document("options").unwrap();
    assert_eq!(options.get_bool("is_consumable").unwrap(), false);
    assert_eq!(options.get_bool("is_consumed").unwrap(), false);
}

#[tokio::test]
async fn v1_without_api_key_is_unauthorized() {
    let app = TestApp::new().await;

    let res = app.post_json("/v1/request/upload", json!({})).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.json(), json!({ "data": null, "message": "Failed to authorize" }));
}

#[tokio::test]
async fn v1_with_unknown_api_key_is_bad_request_with_empty_body() {
    let app = TestApp::new().await;

    let res = app.post_json("/v1/request/upload", json!({ "api_key": "nope" })).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert!(res.body.is_empty());
}

// ---------- v2 ----------

async fn create_v2(app: &TestApp, body: Value) -> common::TestResponse {
    app.post_json("/v2/request/upload", body).await
}

#[tokio::test]
async fn v2_returns_query_style_url() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let res = create_v2(
        &app,
        json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key }),
    )
    .await;

    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    let body = res.json();
    let request_id = body["request_id"].as_str().unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(url.starts_with(&format!("{URL_BASE}/v2/upload?r={request_id}&c=")), "{url}");
    for param in ["&e=", "&n=", "&s="] {
        assert!(url.contains(param), "missing {param} in {url}");
    }
    assert!(body.get("data").is_none(), "v2 is not wrapped in data");
}

#[tokio::test]
async fn v2_stores_all_options() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let res = create_v2(
        &app,
        json!({
            "public_key": bucket.public_key,
            "secret_key": bucket.secret_key,
            "duration": 60,
            "target": "docs",
            "is_consumable": true,
            "is_public": true,
            "size_limit": 1024,
            "mime_types": ["image/png"],
        }),
    )
    .await;
    let request_id = res.json()["request_id"].as_str().unwrap().to_string();

    let stored = app
        .find_one("request", doc! { "_id": ObjectId::parse_str(&request_id).unwrap() })
        .await
        .unwrap();
    assert_eq!(stored.get_object_id("project_id").unwrap().to_hex(), bucket.id);
    assert_eq!(stored.get_str("permission").unwrap(), "upload_v2");
    assert_eq!(stored.get_str("target").unwrap(), "docs");
    let duration = stored.get_i64("expiration_date").unwrap() - stored.get_i64("date_created").unwrap();
    assert_eq!(duration, 60);
    let options = stored.get_document("options").unwrap();
    assert_eq!(options.get_bool("is_consumable").unwrap(), true);
    assert_eq!(options.get_bool("is_consumed").unwrap(), false);
    assert_eq!(options.get_bool("is_public").unwrap(), true);
    assert_eq!(options.get_i64("size_limit").unwrap(), 1024);
    assert_eq!(options.get_array("mime_types").unwrap().len(), 1);
}

#[tokio::test]
async fn v2_without_both_keys_is_unauthorized() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    for body in [
        json!({}),
        json!({ "public_key": bucket.public_key }),
        json!({ "secret_key": bucket.secret_key }),
    ] {
        let res = create_v2(&app, body.clone()).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "body: {body}");
        assert!(res.body.is_empty());
    }
}

#[tokio::test]
async fn v2_with_unknown_public_key_is_bad_request() {
    let app = TestApp::new().await;

    let res = create_v2(&app, json!({ "public_key": "nope", "secret_key": "nope" })).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v2_with_wrong_secret_key_is_unauthorized() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let res = create_v2(&app, json!({ "public_key": bucket.public_key, "secret_key": "wrong" })).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_rejects_unsafe_target() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    for target in ["..", "../x", "../../etc", "..//tmp/x", "images/../../x"] {
        let res = create_v2(
            &app,
            json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key, "target": target }),
        )
        .await;

        assert_eq!(res.status, StatusCode::BAD_REQUEST, "target: {target}");
        assert_eq!(res.json(), json!({ "message": "Invalid target" }));
    }
    assert!(app.find_one("request", doc! {}).await.is_none(), "no request was stored");
}

#[tokio::test]
async fn v2_accepts_slashes_around_target() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let res = create_v2(
        &app,
        json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key, "target": "/images/2024/" }),
    )
    .await;

    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
}

#[tokio::test]
async fn v1_accepts_unsafe_target_and_stores_it_as_given() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;

    let res = app
        .post_json("/v1/request/upload", json!({ "api_key": "k", "target": "../../etc" }))
        .await;

    // v1 is legacy: the target is cleaned when the file is written, not rejected here
    assert_eq!(res.status, StatusCode::CREATED);
    let stored = app.find_one("request", doc! { "permission": "upload_v1" }).await.unwrap();
    assert_eq!(stored.get_str("target").unwrap(), "../../etc");
}
