//! Characterization tests for using upload URLs: v1 path-style URLs and `/v2/upload`.

mod common;

use axum::http::StatusCode;
use common::{multipart_request, path_of, TestApp, PNG};
use mongodb::bson::{doc, oid::ObjectId};
use serde_json::json;

fn oid(hex: &str) -> ObjectId {
    ObjectId::parse_str(hex).unwrap()
}

// ---------- v1 ----------

async fn upload_url_v1(app: &TestApp, body: serde_json::Value) -> String {
    let res = app.post_json("/v1/request/upload", body).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
    path_of(res.json()["data"]["url"].as_str().unwrap())
}

#[tokio::test]
async fn v1_upload_saves_files_and_wraps_result_in_data() {
    let app = TestApp::new().await;
    let project_id = app.insert_legacy_project("legacy", "k").await;
    let url = upload_url_v1(&app, json!({ "api_key": "k" })).await;

    // v1 reads the multipart field "files"
    let res = app.post_multipart(&url, "files", &[("a.png", PNG)]).await;

    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let data = &res.json()["data"];
    assert_eq!(data["path"], format!("/{project_id}/"));
    let file = &data["files"][0];
    let file_id = file["_id"].as_str().unwrap();
    assert_eq!(file["file_name"], "a.png");
    assert_eq!(file["path"], format!("/{project_id}/{file_id}/{file_id}.png"));
    let on_disk = app.data_dir.path().join(&project_id).join(file_id).join(format!("{file_id}.png"));
    assert_eq!(std::fs::read(on_disk).unwrap(), PNG);
}

#[tokio::test]
async fn v1_upload_also_works_under_v1_prefix() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    // generated URLs use the legacy alias `/id/...`; `/v1/id/...` is the same handler
    let url = upload_url_v1(&app, json!({ "api_key": "k" })).await;
    assert!(url.starts_with("/id/"), "{url}");

    let res = app.post_multipart(&format!("/v1{url}"), "files", &[("a.txt", b"hi")]).await;

    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
}

#[tokio::test]
async fn v1_consumable_url_can_be_reused() {
    let app = TestApp::new().await;
    let project_id = app.insert_legacy_project("legacy", "k").await;
    let url = upload_url_v1(&app, json!({ "api_key": "k", "is_consumable": true })).await;

    let first = app.post_multipart(&url, "files", &[("a.txt", b"1")]).await;
    let second = app.post_multipart(&url, "files", &[("b.txt", b"2")]).await;

    assert_eq!(first.status, StatusCode::OK);
    // BUG: v1 marks the request consumed but never checks the flag
    assert_eq!(second.status, StatusCode::OK);
    let request = app.find_one("request", doc! { "project_id": oid(&project_id) }).await.unwrap();
    assert!(request.get_document("options").unwrap().get_bool("is_consumed").unwrap());
}

#[tokio::test]
async fn v1_tampered_signature_is_rejected_as_expired() {
    let app = TestApp::new().await;
    app.insert_legacy_project("legacy", "k").await;
    let url = upload_url_v1(&app, json!({ "api_key": "k" })).await;

    let (start, _) = url.rsplit_once("/signature/").unwrap();
    let res = app
        .post_multipart(&format!("{start}/signature/{}", "0".repeat(64)), "files", &[("a.txt", b"x")])
        .await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request Expired" }));
}

// ---------- v2 ----------

#[tokio::test]
async fn v2_upload_saves_files_and_records() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;
    let request_id = url.split("r=").nth(1).unwrap().split('&').next().unwrap().to_string();

    let res = app
        .post_multipart(&url, "file", &[("a.png", PNG), ("notes.txt", b"hello")])
        .await;

    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
    let files = res.json()["files"].as_array().unwrap().clone();
    assert_eq!(files.len(), 2);
    let png_id = files[0]["_id"].as_str().unwrap();
    assert_eq!(files[0]["path"], format!("/{}/{png_id}/{png_id}.png", bucket.id));

    let record = app.find_one("file", doc! { "_id": oid(png_id) }).await.unwrap();
    assert_eq!(record.get_str("file_name").unwrap(), "a.png");
    assert_eq!(record.get_object_id("project_id").unwrap().to_hex(), bucket.id);
    assert_eq!(record.get_object_id("request_id").unwrap().to_hex(), request_id);
    assert_eq!(record.get_i64("file_size").unwrap(), PNG.len() as i64);
    assert!(!record.get_document("options").unwrap().get_bool("is_public").unwrap());
}

#[tokio::test]
async fn v2_upload_detects_mime_type_and_extension() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let files = app
        .upload_v2(
            &bucket,
            json!({}),
            &[("image.png", PNG), ("notes.txt", b"hello"), ("png-named.txt", PNG), ("README", b"x")],
        )
        .await;

    let mut found = Vec::new();
    for file in &files {
        let record = app.find_one("file", doc! { "_id": oid(file["_id"].as_str().unwrap()) }).await.unwrap();
        found.push((
            record.get_str("file_name").unwrap().to_string(),
            record.get_str("mime_type").ok().map(str::to_string),
            record.get_str("extension").ok().map(str::to_string),
        ));
    }
    let s = |v: &str| Some(v.to_string());
    assert_eq!(
        found,
        vec![
            ("image.png".into(), s("image/png"), s("png")),
            ("notes.txt".into(), s("text/plain"), s("txt")),
            // content wins over the file name
            ("png-named.txt".into(), s("image/png"), s("png")),
            ("README".into(), None, None),
        ]
    );
    // the file on disk is still named after the original extension, not the detected one
    let renamed = files[2]["_id"].as_str().unwrap();
    assert_eq!(files[2]["path"], format!("/{}/{renamed}/{renamed}.txt", bucket.id));
    let no_ext = files[3]["_id"].as_str().unwrap();
    assert_eq!(files[3]["path"], format!("/{}/{no_ext}/{no_ext}", bucket.id));
}

#[tokio::test]
async fn v2_upload_into_target_folder() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let files = app.upload_v2(&bucket, json!({ "target": "/images/2024/" }), &[("a.png", PNG)]).await;

    let id = files[0]["_id"].as_str().unwrap();
    assert_eq!(files[0]["path"], format!("/{}/images/2024/{id}/{id}.png", bucket.id));
}

#[tokio::test]
async fn v1_unsafe_target_is_cleaned_and_stays_inside_data_dir() {
    let app = TestApp::new().await;
    let project_id = app.insert_legacy_project("legacy", "k").await;
    let outside = tempfile::tempdir().unwrap();
    let outside_path = outside.path().display().to_string();

    // "..<abs path>" used to become an absolute path that replaced DATA_DIR entirely
    for target in [format!("..{outside_path}"), outside_path.clone(), format!("../../..{outside_path}")] {
        let url = upload_url_v1(&app, json!({ "api_key": "k", "target": target })).await;
        let res = app.post_multipart(&url, "files", &[("a.txt", b"x")]).await;

        assert_eq!(res.status, StatusCode::OK, "target {target}: {}", res.text());
        let file = &res.json()["data"]["files"][0];
        let id = file["_id"].as_str().unwrap();
        let inside = format!("/{project_id}{outside_path}/{id}/{id}.txt");
        assert_eq!(file["path"], inside, "target: {target}");
        assert!(app.data_dir.path().join(inside.trim_start_matches('/')).is_file());
    }
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0, "nothing written outside");
}

#[tokio::test]
async fn v2_upload_with_stored_unsafe_target_is_rejected() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({ "target": "fine" })).await;
    // simulate a request stored before targets were validated
    app.db
        .collection::<mongodb::bson::Document>("request")
        .update_one(doc! { "project_id": oid(&bucket.id) }, doc! { "$set": { "target": "../escape" } }, None)
        .await
        .unwrap();

    let res = app.post_multipart(&url, "file", &[("a.txt", b"x")]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request could not be completed" }));
    assert!(app.find_one("file", doc! { "file_name": "a.txt" }).await.is_none());
    assert!(!app.data_dir.path().join("escape").exists());
}

#[tokio::test]
async fn v2_upload_ignores_parts_with_other_field_names() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;

    let res = app.post_multipart(&url, "files", &[("a.txt", b"x")]).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json(), json!({ "files": [] }));
}

#[tokio::test]
async fn v2_public_upload_marks_files_public() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let files = app.upload_v2(&bucket, json!({ "is_public": true }), &[("a.txt", b"x")]).await;

    let record = app.find_one("file", doc! { "_id": oid(files[0]["_id"].as_str().unwrap()) }).await.unwrap();
    assert!(record.get_document("options").unwrap().get_bool("is_public").unwrap());
}

#[tokio::test]
async fn v2_mime_types_option_is_not_enforced() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let files = app
        .upload_v2(&bucket, json!({ "mime_types": ["image/png"] }), &[("a.txt", b"not a png")])
        .await;

    assert_eq!(files.len(), 1);
}

#[tokio::test]
async fn v2_size_limit_rejects_request_but_keeps_earlier_files() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({ "size_limit": 10 })).await;

    let res = app
        .post_multipart(&url, "file", &[("small.txt", b"12345"), ("big.txt", &[b'x'; 20])])
        .await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request could not be completed" }));
    // BUG: the first file stays saved (record + disk) although the client got an error
    let orphan = app.find_one("file", doc! { "file_name": "small.txt" }).await.unwrap();
    let id = orphan.get_object_id("_id").unwrap().to_hex();
    assert!(app.data_dir.path().join(&bucket.id).join(&id).join(format!("{id}.txt")).is_file());
    assert!(app.find_one("file", doc! { "file_name": "big.txt" }).await.is_none());
}

#[tokio::test]
async fn v2_size_limit_is_per_file() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;

    let files = app
        .upload_v2(&bucket, json!({ "size_limit": 10 }), &[("a.txt", &[b'x'; 8]), ("b.txt", &[b'x'; 8])])
        .await;

    assert_eq!(files.len(), 2, "16 bytes total is fine when each file is under 10");
}

#[tokio::test]
async fn v2_consumable_url_can_be_reused() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({ "is_consumable": true })).await;

    let first = app.post_multipart(&url, "file", &[("a.txt", b"1")]).await;
    let second = app.post_multipart(&url, "file", &[("b.txt", b"2")]).await;

    assert_eq!(first.status, StatusCode::OK);
    // BUG: the "mark consumed" update_one is never awaited, so it never runs
    assert_eq!(second.status, StatusCode::OK);
    let request = app.find_one("request", doc! { "project_id": oid(&bucket.id) }).await.unwrap();
    assert!(!request.get_document("options").unwrap().get_bool("is_consumed").unwrap());
}

#[tokio::test]
async fn v2_tampered_signature_is_rejected_as_expired() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;

    let (start, _) = url.rsplit_once("&s=").unwrap();
    let res = app.post_multipart(&format!("{start}&s={}", "0".repeat(64)), "file", &[("a.txt", b"x")]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request Expired" }));
}

#[tokio::test]
async fn v2_missing_signature_params_is_rejected_as_expired() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;

    let only_request = url.split('&').next().unwrap();
    let res = app.post_multipart(only_request, "file", &[("a.txt", b"x")]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request Expired" }));
}

#[tokio::test]
async fn v2_upload_panics_without_request_id_or_with_invalid_one() {
    let app = TestApp::new().await;

    // BUG: `query.request.unwrap()` and `ObjectId::from_str(..).unwrap()`
    assert!(app.panics(multipart_request("/v2/upload", "file", &[("a.txt", b"x")])).await);
    assert!(
        app.panics(multipart_request("/v2/upload?r=bad&c=1&e=9999999999&n=1&s=x", "file", &[("a.txt", b"x")]))
            .await
    );
}
