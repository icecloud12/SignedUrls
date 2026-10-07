//! Characterization tests for deleting files: `/delete/:id`, `/v1/delete/:id`, `/v2/delete/:id`.

mod common;

use axum::http::{Method, StatusCode};
use common::{json_request, path_of, TestApp, PNG};
use mongodb::bson::{doc, oid::ObjectId};
use serde_json::json;

fn oid(hex: &str) -> ObjectId {
    ObjectId::parse_str(hex).unwrap()
}

/// Uploads one file into a legacy project through v1 and returns (id, path on disk).
async fn upload_v1(app: &TestApp, api_key: &str, name: &str) -> (String, std::path::PathBuf) {
    let res = app.post_json("/v1/request/upload", json!({ "api_key": api_key })).await;
    let url = path_of(res.json()["data"]["url"].as_str().unwrap());
    let res = app.post_multipart(&url, "files", &[(name, PNG)]).await;
    let file = &res.json()["data"]["files"][0];
    let id = file["_id"].as_str().unwrap().to_string();
    let on_disk = app.data_dir.path().join(file["path"].as_str().unwrap().trim_start_matches('/'));
    (id, on_disk)
}

// ---------- v1 ----------

#[tokio::test]
async fn v1_delete_removes_record_and_file_on_both_routes() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;

    for route in ["/delete", "/v1/delete"] {
        let (id, on_disk) = upload_v1(&app, "k", "a.png").await;
        assert!(on_disk.is_file());

        let res = app.delete_json(&format!("{route}/{id}"), json!({ "api_key": "k" })).await;

        assert_eq!(res.status, StatusCode::OK, "route {route}: {}", res.text());
        assert_eq!(res.json(), json!({ "message": "Successfully removed file" }));
        assert!(app.find_one("file", doc! { "_id": oid(&id) }).await.is_none());
        assert!(!on_disk.exists());
    }
}

#[tokio::test]
async fn v1_delete_errors() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    app.insert_legacy_project("other", "other-key").await;
    let (id, _) = upload_v1(&app, "k", "a.png").await;

    let missing_key = app.delete_json(&format!("/v1/delete/{id}"), json!({})).await;
    assert_eq!(missing_key.status, StatusCode::UNAUTHORIZED);
    assert_eq!(missing_key.json(), json!({ "message": "An unauthorized action" }));

    let unknown_key = app.delete_json(&format!("/v1/delete/{id}"), json!({ "api_key": "nope" })).await;
    assert_eq!(unknown_key.status, StatusCode::BAD_REQUEST);
    assert!(unknown_key.body.is_empty());

    let other_project = app
        .delete_json(&format!("/v1/delete/{id}"), json!({ "api_key": "other-key" }))
        .await;
    assert_eq!(other_project.status, StatusCode::BAD_REQUEST);
    assert_eq!(other_project.json(), json!({ "message": "Parameter mismatch" }));

    let malformed = app.delete_json("/v1/delete/not-an-id", json!({ "api_key": "k" })).await;
    assert_eq!(malformed.status, StatusCode::BAD_REQUEST);
    assert!(malformed.body.is_empty());

    assert!(app.find_one("file", doc! { "_id": oid(&id) }).await.is_some(), "file survived");
}

// ---------- v2 ----------

#[tokio::test]
async fn v2_delete_removes_record_and_file() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let files = app.upload_v2(&bucket, json!({}), &[("a.png", PNG)]).await;
    let id = files[0]["_id"].as_str().unwrap();
    let on_disk = app.data_dir.path().join(files[0]["path"].as_str().unwrap().trim_start_matches('/'));

    let res = app
        .delete_json(
            &format!("/v2/delete/{id}"),
            json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key }),
        )
        .await;

    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    assert_eq!(res.json(), json!({ "message": "Successfully removed file" }));
    assert!(app.find_one("file", doc! { "_id": oid(id) }).await.is_none());
    assert!(!on_disk.exists());
}

#[tokio::test]
async fn v2_delete_of_file_without_extension_fails_after_deleting_record() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let files = app.upload_v2(&bucket, json!({}), &[("README", b"x")]).await;
    let id = files[0]["_id"].as_str().unwrap();
    let on_disk = app.data_dir.path().join(files[0]["path"].as_str().unwrap().trim_start_matches('/'));

    let res = app
        .delete_json(
            &format!("/v2/delete/{id}"),
            json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key }),
        )
        .await;

    // BUG: delete looks for "<id>.README", fails, but the record is already gone
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert!(res.json()["message"].as_str().unwrap().starts_with("Cannot remove file"));
    assert!(app.find_one("file", doc! { "_id": oid(id) }).await.is_none());
    assert!(on_disk.is_file(), "the file itself is left behind");
}

#[tokio::test]
async fn v2_delete_errors() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let other = app.create_bucket("other").await;
    let files = app.upload_v2(&bucket, json!({}), &[("a.png", PNG)]).await;
    let id = files[0]["_id"].as_str().unwrap();
    let path = format!("/v2/delete/{id}");

    let missing_key = app.delete_json(&path, json!({})).await;
    assert_eq!(missing_key.status, StatusCode::UNAUTHORIZED);
    assert_eq!(missing_key.json(), json!({ "message": "An unauthorized action" }));

    let unknown = app.delete_json(&path, json!({ "public_key": "nope", "secret_key": "x" })).await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST);

    let wrong_secret = app
        .delete_json(&path, json!({ "public_key": bucket.public_key, "secret_key": "wrong" }))
        .await;
    assert_eq!(wrong_secret.status, StatusCode::UNAUTHORIZED);

    let other_bucket = app
        .delete_json(&path, json!({ "public_key": other.public_key, "secret_key": other.secret_key }))
        .await;
    assert_eq!(other_bucket.status, StatusCode::BAD_REQUEST);
    assert_eq!(other_bucket.json(), json!({ "message": "Parameter mismatch" }));

    // BUG: a public key without a secret key panics (`secret_key.unwrap()`)
    let no_secret = json_request(Method::DELETE, &path, &json!({ "public_key": bucket.public_key }));
    assert!(app.panics(no_secret).await);

    assert!(app.find_one("file", doc! { "_id": oid(id) }).await.is_some(), "file survived");
}
