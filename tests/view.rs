//! Characterization tests for viewing files:
//! creating view URLs (v1, v2), using them, and public preview.

mod common;

use axum::http::{Method, StatusCode};
use common::{json_request, path_of, Bucket, TestApp, PNG};
use mongodb::bson::doc;
use serde_json::json;

const V1_ROUTES: [&str; 2] = ["/request/create/view", "/v1/request/view"];

/// Uploads one file and returns its id.
async fn upload_one(app: &TestApp, bucket: &Bucket, name: &str, contents: &[u8], public: bool) -> String {
    let files = app.upload_v2(bucket, json!({ "is_public": public }), &[(name, contents)]).await;
    files[0]["_id"].as_str().unwrap().to_string()
}

/// Uploads one file into a legacy project through v1 and returns its id.
async fn upload_one_v1(app: &TestApp, api_key: &str, name: &str, contents: &[u8]) -> String {
    let res = app.post_json("/v1/request/upload", json!({ "api_key": api_key })).await;
    let url = path_of(res.json()["data"]["url"].as_str().unwrap());
    let res = app.post_multipart(&url, "files", &[(name, contents)]).await;
    res.json()["data"]["files"][0]["_id"].as_str().unwrap().to_string()
}

// ---------- create view request: v1 ----------

#[tokio::test]
async fn v1_view_request_returns_base_url_and_object_id_as_extended_json() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;

    for route in V1_ROUTES {
        let res = app
            .post_json(route, json!({ "api_key": "k", "file_id_collection": ["anything"] }))
            .await;

        assert_eq!(res.status, StatusCode::CREATED, "route {route}: {}", res.text());
        let data = &res.json()["data"];
        // request_id is an ObjectId serialized by serde_json: {"$oid": "..."}, not a plain string
        let request_id = data["request_id"]["$oid"].as_str().unwrap();
        let base_url = data["base_url"].as_str().unwrap();
        assert!(base_url.starts_with(&format!("{}/id/{request_id}/permission/view_v1/", common::URL_BASE)));
        assert!(base_url.ends_with("/file/"), "client appends the file id: {base_url}");
    }
}

#[tokio::test]
async fn v1_view_request_does_not_validate_file_ids() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;

    let res = app
        .post_json("/v1/request/view", json!({ "api_key": "k", "file_id_collection": ["not-an-id"] }))
        .await;

    assert_eq!(res.status, StatusCode::CREATED);
    let stored = app.find_one("request", doc! { "permission": "view_v1" }).await.unwrap();
    assert_eq!(stored.get_array("files").unwrap()[0].as_str(), Some("not-an-id"));
}

#[tokio::test]
async fn v1_view_request_errors() {
    let app = TestApp::new().await;

    let missing_key = app.post_json("/v1/request/view", json!({ "file_id_collection": [] })).await;
    assert_eq!(missing_key.status, StatusCode::UNAUTHORIZED);

    // BUG: an unknown api_key becomes 500 (the 400 from key validation is replaced)
    let unknown_key = app
        .post_json("/v1/request/view", json!({ "api_key": "nope", "file_id_collection": [] }))
        .await;
    assert_eq!(unknown_key.status, StatusCode::INTERNAL_SERVER_ERROR);

    // BUG: a missing file list panics (`files.unwrap()`)
    let no_files = json_request(Method::POST, "/v1/request/view", &json!({ "api_key": "nope" }));
    assert!(app.panics(no_files).await);
}

// ---------- create view request: v2 ----------

#[tokio::test]
async fn v2_view_request_returns_url_and_details_per_file() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.png", PNG, false).await;

    let body = app.view_request_v2(&bucket, &[&id], json!({})).await;

    let request_id = body["request_id"]["$oid"].as_str().unwrap();
    let file = &body["files"][0];
    assert_eq!(file["id"], id);
    assert_eq!(file["file_name"], "a.png");
    assert_eq!(file["mime_type"], "image/png");
    assert_eq!(file["extension"], "png");
    assert_eq!(file["file_size"], PNG.len());
    let url = file["url"].as_str().unwrap();
    assert!(url.starts_with(&format!("{}/v2/file/{id}?r={request_id}&c=", common::URL_BASE)), "{url}");
}

#[tokio::test]
async fn v2_view_request_silently_drops_unknown_file_ids() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"x", false).await;

    let body = app
        .view_request_v2(&bucket, &[&id, "not-an-id", "000000000000000000000000"], json!({}))
        .await;

    let files = body["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["id"], id);
}

#[tokio::test]
async fn v2_view_request_errors() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let missing_secret = app
        .post_json("/v2/request/view", json!({ "public_key": bucket.public_key, "files": [] }))
        .await;
    assert_eq!(missing_secret.status, StatusCode::UNAUTHORIZED);

    // BUG: both an unknown public key (400) and a wrong secret (401) become 500
    let unknown = app
        .post_json("/v2/request/view", json!({ "public_key": "nope", "secret_key": "x", "files": [] }))
        .await;
    assert_eq!(unknown.status, StatusCode::INTERNAL_SERVER_ERROR);
    let wrong_secret = app
        .post_json(
            "/v2/request/view",
            json!({ "public_key": bucket.public_key, "secret_key": "wrong", "files": [] }),
        )
        .await;
    assert_eq!(wrong_secret.status, StatusCode::INTERNAL_SERVER_ERROR);

    // BUG: no valid file ids panics (`signatures.get(0).unwrap()`)
    let none_valid = json_request(
        Method::POST,
        "/v2/request/view",
        &json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key, "files": ["000000000000000000000000"] }),
    );
    assert!(app.panics(none_valid).await);
}

// ---------- using view URLs ----------

#[tokio::test]
async fn v1_view_url_returns_file_contents() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    let id = upload_one_v1(&app, "k", "a.txt", b"hello v1").await;
    let res = app
        .post_json("/v1/request/view", json!({ "api_key": "k", "file_id_collection": [id] }))
        .await;
    let base = path_of(res.json()["data"]["base_url"].as_str().unwrap());

    let res = app.get(&format!("{base}{id}")).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(&res.body[..], b"hello v1");
    assert!(res.headers.get("content-type").is_none(), "no content type is sent");
}

#[tokio::test]
async fn v1_view_url_rejects_file_not_in_request() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    let listed = upload_one_v1(&app, "k", "a.txt", b"a").await;
    let other = upload_one_v1(&app, "k", "b.txt", b"b").await;
    let res = app
        .post_json("/v1/request/view", json!({ "api_key": "k", "file_id_collection": [listed] }))
        .await;
    let base = path_of(res.json()["data"]["base_url"].as_str().unwrap());

    let res = app.get(&format!("{base}{other}")).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v1_view_url_with_consumable_can_be_reused() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    let id = upload_one_v1(&app, "k", "a.txt", b"a").await;
    let res = app
        .post_json(
            "/v1/request/view",
            json!({ "api_key": "k", "file_id_collection": [id], "is_consumable": true }),
        )
        .await;
    let url = format!("{}{id}", path_of(res.json()["data"]["base_url"].as_str().unwrap()));

    assert_eq!(app.get(&url).await.status, StatusCode::OK);
    // v1 view requests are never consumed (one signature is shared by all files)
    assert_eq!(app.get(&url).await.status, StatusCode::OK);
}

#[tokio::test]
async fn v2_view_url_returns_file_contents() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.png", PNG, false).await;
    let body = app.view_request_v2(&bucket, &[&id], json!({})).await;

    let res = app.get(&path_of(body["files"][0]["url"].as_str().unwrap())).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(&res.body[..], PNG);
    assert!(res.headers.get("content-type").is_none(), "no content type is sent");
}

#[tokio::test]
async fn v2_view_url_for_file_without_extension() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "README", b"plain", false).await;
    let body = app.view_request_v2(&bucket, &[&id], json!({})).await;

    let res = app.get(&path_of(body["files"][0]["url"].as_str().unwrap())).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(&res.body[..], b"plain");
}

#[tokio::test]
async fn v2_consumable_view_url_works_once() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"x", false).await;
    let body = app.view_request_v2(&bucket, &[&id], json!({ "is_consumable": true })).await;
    let url = path_of(body["files"][0]["url"].as_str().unwrap());

    assert_eq!(app.get(&url).await.status, StatusCode::OK);
    assert_eq!(app.get(&url).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_private_file_without_signature_is_unauthorized() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"x", false).await;

    let res = app.get(&format!("/v2/file/{id}")).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_public_file_needs_no_signature() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"public!", true).await;

    let res = app.get(&format!("/v2/file/{id}")).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(&res.body[..], b"public!");
}

#[tokio::test]
async fn v2_tampered_view_signature_is_unauthorized() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"x", false).await;
    let body = app.view_request_v2(&bucket, &[&id], json!({})).await;
    let url = path_of(body["files"][0]["url"].as_str().unwrap());

    let (start, _) = url.rsplit_once("&s=").unwrap();
    let res = app.get(&format!("{start}&s={}", "0".repeat(64))).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_view_url_for_another_buckets_file_does_not_work() {
    let app = TestApp::new().await;
    let owner = app.create_bucket("owner").await;
    let other = app.create_bucket("other").await;
    let id = upload_one(&app, &owner, "a.txt", b"x", false).await;

    // creating the URL succeeds: file ids aren't checked against the bucket
    let body = app.view_request_v2(&other, &[&id], json!({})).await;
    let url = path_of(body["files"][0]["url"].as_str().unwrap());

    // but it's signed with the wrong bucket id, so using it fails
    assert_eq!(app.get(&url).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_view_of_unknown_file_is_unauthorized() {
    let app = TestApp::new().await;

    assert_eq!(app.get("/v2/file/000000000000000000000000").await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.get("/v2/file/not-an-id").await.status, StatusCode::UNAUTHORIZED);
}

// ---------- public preview ----------

#[tokio::test]
async fn preview_returns_public_file_on_both_routes() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"preview me", true).await;

    for route in ["/preview", "/v1/preview"] {
        let res = app.get(&format!("{route}/{id}")).await;
        assert_eq!(res.status, StatusCode::OK, "route {route}");
        assert_eq!(&res.body[..], b"preview me");
    }
}

#[tokio::test]
async fn preview_of_private_file_is_not_found() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "a.txt", b"x", false).await;

    let res = app.get(&format!("/preview/{id}")).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.text(), "File not found");
}

#[tokio::test]
async fn preview_of_public_file_without_extension_is_not_found() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let id = upload_one(&app, &bucket, "README", b"x", true).await;

    let res = app.get(&format!("/preview/{id}")).await;

    // BUG: preview always looks for "<id>.<last part of name>", here "<id>.README"
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert!(res.text().starts_with("File not found: "), "{}", res.text());
}

#[tokio::test]
async fn preview_errors() {
    let app = TestApp::new().await;

    let unknown = app.get("/preview/000000000000000000000000").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.text(), "file not found");

    // BUG: a malformed id is a client error, not 500
    let malformed = app.get("/preview/not-an-id").await;
    assert_eq!(malformed.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(malformed.text(), "Internal server error");
}
