use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;

pub trait AppError {
    fn status(&self) -> StatusCode;
    fn code(&self) -> String;
    fn message(&self) -> &'static str;
}
pub enum BucketError {
    AlreadyExists,
    CannotWriteDirectory, //either path does not exist or having no permission for writes
}

impl AppError for BucketError {
    fn status(&self) -> StatusCode {
        match self {
            Self::AlreadyExists => StatusCode::CONFLICT,
            Self::CannotWriteDirectory => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn code(&self) -> String {
        match self {
            Self::AlreadyExists => "BUCKET_ALREADY_EXISTS".into(),
            Self::CannotWriteDirectory => "INTERNAL_ERROR".into(),
        }
    }
    fn message(&self) -> &'static str {
        match self {
            Self::AlreadyExists => "Bucket already exists",
            Self::CannotWriteDirectory => "Something went wrong when trying to create the bucket",
        }
    }
}

impl IntoResponse for BucketError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status(),
            Json(json!({"code": self.code(), "message": self.message()})),
        )
            .into_response()
    }
}
