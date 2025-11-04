use std::str::FromStr;
use mongodb::{bson::doc};
use axum::{extract::{Path, Query}, response::IntoResponse};
use hyper::StatusCode;
use mongodb::{bson::oid::ObjectId, Database};
use signed_urls::collections;

use crate::{
    network::{db_connection::DATABASE, DbCollection},
    models::request_models::{RequestQueryParamsV2, ViewRequest},
    signed_url::{
        actions::{validate_signed_url_v1, validate_signed_url_v2, ActionTypes},
        handlers::read_file
    }
};


pub async fn process_signed_url_view_request_v1(
    Path(params): Path<Vec<(String, String)>>,
) -> impl IntoResponse {
    let collected_params: Vec<String> = params
        .iter()
        .map(|param| param.1.clone())
        .collect::<Vec<String>>();
    let request_id: ObjectId = ObjectId::from_str(collected_params[0].as_str()).unwrap();
    let file_id = collected_params[5].clone();
    //[request_id, created, expiration, nonce, signature, file] -- tho only signature is used in valdiating signed URL
    if validate_signed_url_v1(collected_params, ActionTypes::VIEW_V1.to_string().as_str()).await {
        //if validated check if request view document has file
        let db: &Database = DATABASE.get().unwrap();
        let view_request_document_result = db
            .collection::<ViewRequest>(DbCollection::REQUEST.to_string().as_str())
            .find_one(
                doc! { collections::ViewFileRequest::REQUEST_ID: request_id},
                None,
            )
            .await
            .unwrap()
            .unwrap(); // we shouldn't technically touch the database directly so we can simply assume thigns would work out
        if view_request_document_result.files.contains(&file_id) {
            //file_id is in the list. check if it is a valid file_id referenec
            let (status_code, body) = read_file(&file_id, db).await;
            if status_code == StatusCode::OK {
                return (status_code, body.unwrap()).into_response();
            } else {
                return (status_code).into_response();
            }
        } else {
            return (StatusCode::UNAUTHORIZED).into_response();
        };
    } else {
        return (StatusCode::BAD_REQUEST).into_response();
    }
}
pub async fn process_signed_url_view_request_v2(
    Path(params): Path<Vec<(String, String)>>,
    query: Query<RequestQueryParamsV2>,
) -> impl IntoResponse {
    let db: &Database = DATABASE.get().unwrap();
    if params.len() == 1 {
        let (_file_param_key, file_id) = params.get(0).unwrap();
        if validate_signed_url_v2(Some(file_id), query.0, ActionTypes::VIEW_V2).await {
            let (status_code, body) = read_file(file_id, db).await;
            if status_code == StatusCode::OK {
                return (status_code, body.unwrap()).into_response();
            } else {
                return (status_code).into_response();
            }
        } else {
            return (StatusCode::UNAUTHORIZED).into_response();
        }
    } else {
        return (StatusCode::UNAUTHORIZED).into_response();
        //return bad request
        // return (StatusCode::BAD_REQUEST).into_response();
    }
}
