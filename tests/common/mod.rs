//! Shared setup for integration tests.
//!
//! Each `TestApp` gets its own MongoDB database and its own temporary `DATA_DIR`,
//! so tests can run in parallel without seeing each other's data.

// Each file in `tests/` compiles this module separately, and not every test file
// uses every helper, so unused ones would otherwise warn.
#![allow(dead_code)]

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use mongodb::{
    bson::{doc, Document},
    Database,
};
use serde_json::{json, Value};
use signed_urls::{AppState, Config};
use tempfile::TempDir;
use tower::ServiceExt;

pub const PREFIX: &str = "/api";
/// What generated URLs start with (`https://` + `replaced_url` + prefix).
pub const URL_BASE: &str = "https://localhost:8080/api";

/// Local MongoDB used by tests. Override with `TEST_DATABASE_URI`.
fn database_uri() -> String {
    std::env::var("TEST_DATABASE_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into())
}

pub struct TestApp {
    pub router: Router,
    pub db: Database,
    pub data_dir: TempDir,
}

impl TestApp {
    pub async fn new() -> Self {
        let db_name = format!("signed_urls_test_{}", uuid::Uuid::new_v4().simple());
        let data_dir = tempfile::tempdir().unwrap();
        let config = Config {
            address: "127.0.0.1:0".parse().unwrap(),
            prefix: PREFIX.into(),
            replaced_url: "localhost:8080".into(),
            database_uri: database_uri(),
            database_name: db_name,
            default_duration_secs: 3600,
            data_dir: data_dir.path().to_path_buf(),
            tls_files: None,
        };
        let db = signed_urls::connect(&config.database_uri, &config.database_name)
            .await
            .unwrap();
        let router = signed_urls::app(AppState::new(db.clone(), config));
        Self {
            router,
            db,
            data_dir,
        }
    }

    /// Sends a request straight into the router (no server, no TLS).
    pub async fn send(&self, request: Request<Body>) -> TestResponse {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        TestResponse {
            status,
            headers,
            body,
        }
    }

    /// Returns true if the handler panics on this request.
    ///
    /// axum doesn't catch panics, so in production the connection is just dropped.
    /// Running it in a spawned task turns the panic into a `JoinError` we can check.
    pub async fn panics(&self, request: Request<Body>) -> bool {
        let router = self.router.clone();
        let result = tokio::spawn(async move { router.oneshot(request).await }).await;
        matches!(result, Err(e) if e.is_panic())
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        self.send(get_request(path)).await
    }

    pub async fn post_json(&self, path: &str, body: Value) -> TestResponse {
        self.json_request(Method::POST, path, body).await
    }

    pub async fn delete_json(&self, path: &str, body: Value) -> TestResponse {
        self.json_request(Method::DELETE, path, body).await
    }

    async fn json_request(&self, method: Method, path: &str, body: Value) -> TestResponse {
        self.send(json_request(method, path, &body)).await
    }

    /// Uploads `files` as `(file_name, contents)` under the multipart field `field`.
    pub async fn post_multipart(
        &self,
        path: &str,
        field: &str,
        files: &[(&str, &[u8])],
    ) -> TestResponse {
        self.send(multipart_request(path, field, files)).await
    }

    /// Creates a bucket through `/v2/bucket`.
    pub async fn create_bucket(&self, name: &str) -> Bucket {
        let res = self.post_json("/v2/bucket", json!({ "name": name })).await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
        let body = res.json();
        Bucket {
            id: body["_id"].as_str().unwrap().to_string(),
            public_key: body["public_key"].as_str().unwrap().to_string(),
            secret_key: body["secret_key"].as_str().unwrap().to_string(),
        }
    }

    /// Inserts an old-style project (plaintext `api_key`), the only kind v1 routes accept.
    /// New buckets can't be used with v1 because they have no `api_key`.
    pub async fn insert_legacy_project(&self, name: &str, api_key: &str) -> String {
        let id = self
            .db
            .collection::<Document>("project")
            .insert_one(doc! { "name": name, "api_key": api_key }, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .to_hex();
        std::fs::create_dir_all(self.data_dir.path().join(&id)).unwrap();
        id
    }

    /// Creates a v2 upload URL; `options` is merged into the request body.
    pub async fn upload_url_v2(&self, bucket: &Bucket, options: Value) -> String {
        let mut body = json!({ "public_key": bucket.public_key, "secret_key": bucket.secret_key });
        merge(&mut body, options);
        let res = self.post_json("/v2/request/upload", body).await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
        path_of(res.json()["url"].as_str().unwrap())
    }

    /// Uploads files with a fresh v2 upload URL and returns the `files` array.
    pub async fn upload_v2(&self, bucket: &Bucket, options: Value, files: &[(&str, &[u8])]) -> Vec<Value> {
        let url = self.upload_url_v2(bucket, options).await;
        let res = self.post_multipart(&url, "file", files).await;
        assert_eq!(res.status, StatusCode::OK, "{}", res.text());
        res.json()["files"].as_array().unwrap().clone()
    }

    /// Creates a v2 view request for `files` and returns the response body.
    pub async fn view_request_v2(&self, bucket: &Bucket, files: &[&str], options: Value) -> Value {
        let mut body = json!({
            "public_key": bucket.public_key,
            "secret_key": bucket.secret_key,
            "files": files,
        });
        merge(&mut body, options);
        let res = self.post_json("/v2/request/view", body).await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.text());
        res.json()
    }

    pub async fn find_one(&self, collection: &str, filter: Document) -> Option<Document> {
        self.db
            .collection::<Document>(collection)
            .find_one(filter, None)
            .await
            .unwrap()
    }
}

pub struct Bucket {
    pub id: String,
    pub public_key: String,
    pub secret_key: String,
}

/// Turns a generated URL into a path for the request helpers (which add the prefix).
pub fn path_of(url: &str) -> String {
    url.strip_prefix(URL_BASE)
        .unwrap_or_else(|| panic!("unexpected URL: {url}"))
        .to_string()
}

fn merge(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        target.extend(extra);
    }
}

pub fn json_request(method: Method, path: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("{PREFIX}{path}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn get_request(path: &str) -> Request<Body> {
    Request::get(format!("{PREFIX}{path}"))
        .body(Body::empty())
        .unwrap()
}

/// Builds a `multipart/form-data` body by hand.
pub fn multipart_request(path: &str, field: &str, files: &[(&str, &[u8])]) -> Request<Body> {
    const BOUNDARY: &str = "test-boundary-7MA4YWxkTrZu0gW";
    let mut body = Vec::new();
    for (file_name, contents) in files {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(contents);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    Request::post(format!("{PREFIX}{path}"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

/// Drops the test database even when the test panics.
///
/// `Drop` can't be async, so this starts a short-lived thread with its own
/// runtime and MongoDB client just to run the drop.
impl Drop for TestApp {
    fn drop(&mut self) {
        let db_name = self.db.name().to_string();
        let result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                signed_urls::connect(&database_uri(), &db_name)
                    .await?
                    .drop(None)
                    .await
            })
        })
        .join();
        if !matches!(result, Ok(Ok(()))) {
            eprintln!("warning: could not drop test database {}", self.db.name());
        }
    }
}

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| {
            panic!(
                "response is not JSON ({e}): {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Smallest byte string `infer` recognizes as a PNG.
pub const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

