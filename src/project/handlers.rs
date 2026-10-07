use axum::extract::{Json, State};
use axum::response::IntoResponse;
use hyper::StatusCode;
use serde::Deserialize;
use serde_json::json;

use super::actions;
use crate::state::AppState;
#[derive(Deserialize)]
pub struct CreateProjectPostRequest {
    pub name: String,
}
//
pub async fn create_bucket(
    State(state): State<AppState>,
    Json(post_request): Json<CreateProjectPostRequest>,
) -> impl IntoResponse {
    let create_request = actions::create_bucket(&state.db, post_request.name).await;

    match create_request {
        Ok(new_bucket) => {
            //then create directory
            actions::create_bucket_directory(&state.config.data_dir, &new_bucket._id).await;

            return (StatusCode::CREATED, Json(new_bucket)).into_response();
        }
        Err(_) => {
            let j = json!({"message":"Project already exists"});
            return (StatusCode::CONFLICT, Json(j)).into_response();
        }
    };
}

// pub async fn create_bucket_v2(
//     Json(post_request): JSON<CreateProjectPostRequest>,
// ) -> impl Intoresponse {
// }
