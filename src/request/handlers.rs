
use std::str::FromStr;

use super::actions::file_reference_is_valid;
use crate::network::db_connection::DATABASE;
use crate::project::actions::validate_api_key;
use crate::request::model::{CreateSignedUrlPostRequest, CreateSignedUrlPostRequestV2};
use crate::{signed_url::actions::ActionTypes};
use axum::{
    body::Body,
    extract::{Json, Path},
    response::IntoResponse,
};
use hyper::StatusCode;
use mongodb::{bson::oid::ObjectId};
use serde_json::json;
use tokio_util::io::ReaderStream;

pub async fn create_upload_request(
    Json(post_request): Json<CreateSignedUrlPostRequest>,
) -> impl IntoResponse {
    let CreateSignedUrlPostRequest {
        duration,
        is_consumable,
        target,
        is_public,
        api_key,
    } = post_request;
    if api_key.is_some() {
        let db = DATABASE.get().unwrap();
        let api_key = api_key.unwrap();
        match validate_api_key(&db, &api_key, None, ActionTypes::UPLOAD_V1).await {
            Ok(bucket_o)=>{
                match bucket_o{
                    Some(bucket) => {
                        let upload_request_v2 = CreateSignedUrlPostRequestV2{
                            duration,
                            is_consumable,
                            target: target.clone(),
                            is_public,
                            public_key: Some(api_key),
                            secret_key: None,
                            size_limit: None,
                            mime_types: None,
                        };
                        return crate::request::actions::create_upload_request(&db, bucket, upload_request_v2, ActionTypes::UPLOAD_V1).await;
                    }
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!(
                                {"data":None::<String>, "message":"Failed to create signed-url"}
                            )),
                        ).into_response();
                    }    
                }
            }
            Err(status_code)=>{
                return (status_code).into_response();
            }
            
        }
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!(
                {"data":None::<String>, "message":"Failed to authorize"}
            )),
        ).into_response();
    }
}

pub async fn create_upload_request_v2(Json(post_request): Json<CreateSignedUrlPostRequestV2>) -> impl IntoResponse{
    let CreateSignedUrlPostRequestV2 { duration, target, is_consumable, is_public, public_key, secret_key, size_limit, mime_types } = post_request;
    let db = DATABASE.get().unwrap();
    match (public_key, secret_key){
        (Some(public_key_u), Some(secret_key_u))=>{
            match  validate_api_key(&db, &public_key_u, Some(secret_key_u.clone()), ActionTypes::UPLOAD_V2).await{
                Ok(bucket_o)=>{
                    match bucket_o {
                        Some(bucket) => {
                            let upload_request_v2 = CreateSignedUrlPostRequestV2{
                                duration,
                                is_consumable,
                                target,
                                is_public,
                                public_key: Some(public_key_u.to_string()),
                                secret_key: Some(secret_key_u.to_string()),
                                size_limit,
                                mime_types
                            };
                            return crate::request::actions::create_upload_request(&db, bucket, upload_request_v2, ActionTypes::UPLOAD_V2).await
                        }
                        None => {
                            return (StatusCode::BAD_REQUEST).into_response();
                        }
                    }
                }
                Err(status_code) => { return (status_code).into_response();}
            }
        }
        (_, _) => {
            return (StatusCode::UNAUTHORIZED).into_response();
        }
    }
}


pub async fn process_public_read_access(
    Path(params): Path<Vec<(String, String)>>,
) -> impl IntoResponse {
    let file_id = params[0].1.clone();
    //check if file is public
    let file_object_id = ObjectId::from_str(&file_id);
    if file_object_id.is_err() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal server error"),
        ));
    } else {
        match file_reference_is_valid(file_object_id.unwrap()).await {
            Ok(file_document) => {
                if file_document.options.is_public {
                    let initial_path = std::path::PathBuf::from(file_document.path.clone());
                    let file_ext = file_document.file_name.split(".").last().unwrap();
                    let file_document_id = file_document._id.to_hex();
                    let path = initial_path.join(format!(
                        "{}/{}.{}",
                        file_document_id, file_document_id, file_ext
                    ));
                    println!(
                        "filePath: {}",
                        &path.as_os_str().to_str().unwrap().to_string()
                    );
                    let file = match tokio::fs::File::open(path).await {
                        Ok(file) => file,
                        Err(err) => {
                            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)))
                        }
                    };
                    // convert the `AsyncRead` into a `Stream`
                    let stream = ReaderStream::new(file);
                    // convert the `Stream` into an `axum::body::HttpBody`
                    let body = Body::from_stream(stream);
                    return Ok((StatusCode::OK, body).into_response());
                } else {
                    return Err((StatusCode::NOT_FOUND, format!("File not found")));
                }
            }
            Err(response) => return Err(response),
        }
    }
}

