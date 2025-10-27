use std::env;

use crate::network::db_connection::DATABASE;
use crate::project::models::BucketDocument;
use crate::request::model::{CreateSignedUrlPostRequestV2, RequestOptions, UploadRequest};
use crate::signed_url::actions::{create_hashed_signature, ActionTypes};
use crate::{file::model::FileDocument, network::DbCollection};
use axum::response::{IntoResponse, Response};
use axum::Json;
use hyper::StatusCode;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use serde_json::json;
pub async fn file_reference_is_valid(
    file_id: ObjectId,
) -> Result<FileDocument, (StatusCode, String)> {
    let db = DATABASE.get().unwrap();
    let file_ref: Result<Option<FileDocument>, mongodb::error::Error> = db
        .collection::<FileDocument>(DbCollection::FILE.to_string().as_str())
        .find_one(doc! { "_id": file_id}, None)
        .await;
    match file_ref {
        Ok(mongodb_ok) => match mongodb_ok {
            Some(file_doc) => {
                return Ok(file_doc);
            }
            None => return Err((StatusCode::NOT_FOUND, "file not found".to_string())),
        },
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error"),
            ))
        }
    }
}

pub async fn create_upload_request(
    bucket: BucketDocument,
    upload_request: CreateSignedUrlPostRequestV2,
    action_type: ActionTypes,
) -> Response<axum::body::Body> {
    let CreateSignedUrlPostRequestV2 {
        duration,
        target,
        is_consumable,
        is_public,
        public_key: _,
        secret_key: _,
        size_limit,
        mime_types,
    } = upload_request;
    let hashed_signature = create_hashed_signature(
        &bucket._id.to_hex(),
        &duration.unwrap_or_else(|| {
            std::env::var("DEFAULT_DURATION_AS_SECONDS")
                .unwrap()
                .to_string()
                .parse::<u64>()
                .unwrap()
        }),
        &action_type.to_string(),
        None,
    );

    let doc: UploadRequest = UploadRequest {
        project_id: bucket._id,
        date_created: hashed_signature.date_created.clone(),
        expiration_date: hashed_signature.expiration_date.clone(),
        options: RequestOptions {
            is_consumable,
            is_consumed: Some(false),
            is_public,
            size_limit,
            mime_types,
        },
        permission: action_type.to_string(),
        target: target,
    };

    let db: &Database = DATABASE.get().unwrap();
    let insert_request_id = &db
        .collection::<UploadRequest>(DbCollection::REQUEST.to_string().as_str())
        .insert_one(doc, None)
        .await
        .unwrap()
        .inserted_id
        .as_object_id()
        .unwrap()
        .to_string();
    let prefix = env::var("PREFIX").unwrap();
    let replaced_url = env::var("REPLACED_URL").unwrap();
    let generated_url: String = format!(
        "https://{}{}/v2/upload?r={}&c={}&e={}&n={}&s={}",
        replaced_url,
        prefix,
        insert_request_id,
        hashed_signature.date_created,
        hashed_signature.expiration_date,
        hashed_signature.nonce,
        hashed_signature.hashed_signature_base_64
    );
    match action_type {
        ActionTypes::UPLOAD_V1 => {
            return (
                StatusCode::CREATED,
                Json(json!(
                    {"data":{
                        "request_id": insert_request_id,
                        "url": generated_url
                    }
                })),
            )
                .into_response();
        }
        ActionTypes::UPLOAD_V2 => {
            return (
                StatusCode::CREATED,
                Json(json!(
                    {
                        "request_id": insert_request_id,
                        "url": generated_url
                    }
                )),
            )
                .into_response();
        }
        _ => {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
