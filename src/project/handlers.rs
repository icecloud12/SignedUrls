use axum::extract::{Json, State};
use axum::response::IntoResponse;
use hyper::StatusCode;
use serde::Deserialize;

use super::actions;
use crate::error::BucketError;
use crate::state::AppState;
#[derive(Deserialize)]
pub struct CreateProjectPostRequest {
    pub name: String,
}
//
pub async fn create_bucket(
    State(state): State<AppState>,
    Json(post_request): Json<CreateProjectPostRequest>,
) -> Result<impl IntoResponse, BucketError> {
    let create_request = actions::create_bucket(&state.db, post_request.name).await;

    match create_request {
        Ok(new_bucket) => {
            let _ = actions::create_bucket_directory(&state.config.data_dir, &new_bucket._id).await;
            // NOTE
            // a failure on creating a bucket directory is still OK for now since uploading file can create the bucket directory anyway.

            // NOTE
            // if there is a directory issue, we can't give an INTERNAL_SERVER_ERROR since the bucket is written in the db
            Ok((StatusCode::CREATED, Json(new_bucket)))
        }
        Err(_) => Err(BucketError::AlreadyExists),
    }
}
