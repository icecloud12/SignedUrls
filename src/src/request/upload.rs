use crate::{
    models::{
        project_models::BucketDocument,
        request_models::{CreateSignedUrlPostRequestV1, CreateSignedUrlPostRequestV2},
    },
    network::db_connection::DATABASE,
    project::actions::{validate_api_key_v1, validate_api_key_v2},
    signed_url::actions::ActionTypes,
};
use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;

pub async fn create_upload_request_v1(
    Json(post_request): Json<CreateSignedUrlPostRequestV1>,
) -> impl IntoResponse {
    let CreateSignedUrlPostRequestV1 {
        duration,
        is_consumable,
        target,
        is_public,
        api_key,
    } = post_request;
    if api_key.is_some() {
        let db = DATABASE.get().unwrap();
        let api_key = api_key.unwrap();
        match validate_api_key_v1(&db, &api_key).await {
            Ok(project_o) => match project_o {
                Some(project) => {
                    let upload_request_v2 = CreateSignedUrlPostRequestV2 {
                        duration,
                        is_consumable,
                        target: target.clone(),
                        is_public,
                        public_key: Some(api_key),
                        secret_key: None,
                        size_limit: None,
                        mime_types: None,
                    };
                    let bucket = BucketDocument {
                        _id: project._id,
                        name: project.name,
                        public_key: project.api_key,
                        hashed_secret_key: None,
                        secret_key_hint: None,
                    };
                    return crate::request::actions::create_upload_request(
                        &db,
                        bucket,
                        upload_request_v2,
                        ActionTypes::UPLOAD_V1,
                    )
                    .await;
                }
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!(
                            {"data":None::<String>, "message":"Failed to create signed-url"}
                        )),
                    )
                        .into_response();
                }
            },
            Err(status_code) => {
                return (status_code).into_response();
            }
        }
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!(
                {"data":None::<String>, "message":"Failed to authorize"}
            )),
        )
            .into_response();
    }
}

pub async fn create_upload_request_v2(
    Json(post_request): Json<CreateSignedUrlPostRequestV2>,
) -> impl IntoResponse {
    tracing::info!("here");
    let CreateSignedUrlPostRequestV2 {
        duration,
        target,
        is_consumable,
        is_public,
        public_key,
        secret_key,
        size_limit,
        mime_types,
    } = post_request;
    let db = DATABASE.get().unwrap();
    match (public_key, secret_key) {
        (Some(public_key_u), Some(secret_key_u)) => {
            match validate_api_key_v2(&db, &public_key_u, Some(secret_key_u.clone())).await {
                Ok(bucket_o) => match bucket_o {
                    Some(bucket) => {
                        let upload_request_v2 = CreateSignedUrlPostRequestV2 {
                            duration,
                            is_consumable,
                            target,
                            is_public,
                            public_key: Some(public_key_u.to_string()),
                            secret_key: Some(secret_key_u.to_string()),
                            size_limit,
                            mime_types,
                        };
                        return crate::request::actions::create_upload_request(
                            &db,
                            bucket,
                            upload_request_v2,
                            ActionTypes::UPLOAD_V2,
                        )
                        .await;
                    }
                    None => {
                        tracing::error!("No matching bucket");
                        return (StatusCode::BAD_REQUEST).into_response();
                    }
                },
                Err(status_code) => {
                    return (status_code).into_response();
                }
            }
        }
        (_, _) => {
            return (StatusCode::UNAUTHORIZED).into_response();
        }
    }
}
