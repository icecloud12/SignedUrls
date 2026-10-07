//! Characterization tests for how signatures are built and checked.
//!
//! The signature is SHA-256 over values the client can see or choose, with no
//! server secret. These tests recompute it on the client side to show that.

mod common;

use axum::http::StatusCode;
use common::{TestApp, PNG};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Same steps as `hash_parameters` in `src/signed_url/actions.rs`.
fn sign(bucket_id: &str, created: u64, expiration: u64, permission: &str, nonce: u64, file_id: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bucket_id.as_bytes());
    hasher.update(created.to_be_bytes());
    hasher.update(expiration.to_be_bytes());
    hasher.update(permission.as_bytes());
    hasher.update(nonce.to_be_bytes());
    if let Some(file_id) = file_id {
        hasher.update(file_id.as_bytes());
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn query_param<'a>(url: &'a str, key: &str) -> &'a str {
    url.split(['?', '&'])
        .find_map(|pair| pair.strip_prefix(&format!("{key}=")))
        .unwrap()
}

#[tokio::test]
async fn server_signature_matches_client_side_recomputation() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;

    let expected = sign(
        &bucket.id,
        query_param(&url, "c").parse().unwrap(),
        query_param(&url, "e").parse().unwrap(),
        "upload_v2",
        query_param(&url, "n").parse().unwrap(),
        None,
    );

    assert_eq!(query_param(&url, "s"), expected);
}

#[tokio::test]
async fn forged_upload_signature_with_longer_expiry_is_accepted() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({ "duration": 60 })).await;
    let request_id = query_param(&url, "r");

    // anyone who knows the bucket id can sign their own expiry (here: about 10 years)
    let (created, expiration, nonce) = (now() - 10, now() + 315_360_000, 42);
    let forged = sign(&bucket.id, created, expiration, "upload_v2", nonce, None);
    let res = app
        .post_multipart(
            &format!("/v2/upload?r={request_id}&c={created}&e={expiration}&n={nonce}&s={forged}"),
            "file",
            &[("a.txt", b"x")],
        )
        .await;

    // BUG (security): signatures need a server-side secret (e.g. HMAC with SECRET_KEY)
    assert_eq!(res.status, StatusCode::OK, "{}", res.text());
}

#[tokio::test]
async fn forged_view_signature_for_private_file_is_accepted() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let files = app.upload_v2(&bucket, json!({}), &[("secret.png", PNG)]).await;
    let file_id = files[0]["_id"].as_str().unwrap();
    // a real view request must exist (its id is checked), but its signature doesn't matter
    let body = app.view_request_v2(&bucket, &[file_id], json!({})).await;
    let request_id = body["request_id"]["$oid"].as_str().unwrap();

    let (created, expiration, nonce) = (now() - 10, now() + 315_360_000, 7);
    let forged = sign(&bucket.id, created, expiration, "view_v2", nonce, Some(file_id));
    let res = app
        .get(&format!("/v2/file/{file_id}?r={request_id}&c={created}&e={expiration}&n={nonce}&s={forged}"))
        .await;

    // BUG (security): same as above
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(&res.body[..], PNG);
}

#[tokio::test]
async fn correctly_signed_but_expired_upload_url_is_rejected() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;
    let request_id = query_param(&url, "r");

    let (created, expiration, nonce) = (now() - 120, now() - 60, 1);
    let signature = sign(&bucket.id, created, expiration, "upload_v2", nonce, None);
    let res = app
        .post_multipart(
            &format!("/v2/upload?r={request_id}&c={created}&e={expiration}&n={nonce}&s={signature}"),
            "file",
            &[("a.txt", b"x")],
        )
        .await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "data": "Request Expired" }));
}

#[tokio::test]
async fn correctly_signed_url_from_the_future_is_rejected() {
    let app = TestApp::new().await;
    let bucket = app.create_bucket("b").await;
    let url = app.upload_url_v2(&bucket, json!({})).await;
    let request_id = query_param(&url, "r");

    let (created, expiration, nonce) = (now() + 60, now() + 120, 1);
    let signature = sign(&bucket.id, created, expiration, "upload_v2", nonce, None);
    let res = app
        .post_multipart(
            &format!("/v2/upload?r={request_id}&c={created}&e={expiration}&n={nonce}&s={signature}"),
            "file",
            &[("a.txt", b"x")],
        )
        .await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}
