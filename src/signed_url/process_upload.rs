use std::str::FromStr;

use axum::{extract::{Multipart, Path, Query}, response::IntoResponse, Json};
use hyper::StatusCode;
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use serde_json::json;
use crate::{
    network::{db_connection::DATABASE, DbCollection},
    models::project_models::{BucketDocument, ProjectDocument},
    models::request_models::{RequestQueryParamsV2, UploadRequestDocument, UploadRequestWithBucketDocument},
    signed_url::actions::{save_files_to_directory, validate_signed_url_v1, validate_signed_url_v2, ActionTypes, UploadActionTypes}};

pub async fn process_signed_url_upload_request_v1(
    Path(params): Path<Vec<(String, String)>>,
    multipart: Multipart,
) -> impl IntoResponse {
    //validate the url
    let collected_params = params
        .iter()
        .map(|param| param.1.clone())
        .collect::<Vec<String>>();
    let request_id: String = collected_params[0].clone();
    
    if validate_signed_url_v1(collected_params, ActionTypes::UPLOAD_V1.to_string().as_str()).await {
        let db: &Database = DATABASE.get().unwrap();
        let request = db.collection::<UploadRequestDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": ObjectId::from_str(request_id.as_str()).unwrap()
        }, None).await.unwrap().unwrap();
        let project = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": request.project_id
        },None).await.unwrap().unwrap();
        let save_files_to_directory_result = save_files_to_directory(&request._id.to_hex(), &project._id.to_hex(), request.target, Some(request.options), multipart, UploadActionTypes::try_from(ActionTypes::UPLOAD_V1).unwrap()).await;
        match save_files_to_directory_result {
            Ok(res) => {
                return (StatusCode::OK, Json(json!({"data":res})));
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"data":"Request could not be completed"})),
                );
            }
        }
    }
    return (
        StatusCode::BAD_REQUEST,
        Json(json!({"data":"Request Expired"})),
    );
}

pub async fn process_signed_url_upload_request_v2(
    query: Query<RequestQueryParamsV2>,
    multipart: Multipart,
)-> impl IntoResponse{
    let query = query.0;
    let request_id = query.request.clone();
    tracing::info!("request_id:{:#?}", request_id);
    if validate_signed_url_v2(None, query, ActionTypes::UPLOAD_V2).await{

        let db: &Database = DATABASE.get().unwrap();
        let aggregate_pipeline = vec![
            doc!{ "$lookup": {
                    "from": DbCollection::BUCKET,
                    "localField": signed_urls::collections::Request::PROJECT_ID,
                    "foreignField": signed_urls::collections::Bucket::ID,
                    "as": "bucket"
                }
            },
            doc!{ "$unwind": "$bucket" }
        ];
        match db.collection::<UploadRequestWithBucketDocument>(DbCollection::REQUEST.to_string().as_str()).aggregate(aggregate_pipeline, None).await {
              Err(_)=> { return (StatusCode::SERVICE_UNAVAILABLE).into_response()}
              Ok(mut cursor_document)=>{
                  //expecting only 1 result
                  match cursor_document.advance().await {
                    Err(_)=>{return (StatusCode::SERVICE_UNAVAILABLE).into_response()}                      
                    Ok(_)=>{
                        match cursor_document.with_type().deserialize_current() {
                            Ok(document) => {
                                let UploadRequestWithBucketDocument { _id: request_id, target, options, bucket: BucketDocument{ _id: bucket_id, .. }, .. } = document;

                                let save_files_to_directory_result = save_files_to_directory(&request_id.to_hex(), &bucket_id.to_hex(), target, options, multipart, UploadActionTypes::try_from(ActionTypes::UPLOAD_V2).unwrap()).await;
                                match save_files_to_directory_result {
                                    Ok(res) => {
                                        return (StatusCode::OK, Json(json!({"data":res}))).into_response();
                                    }
                                    Err(_) => {
                                        return (
                                            StatusCode::BAD_REQUEST,
                                            Json(json!({"data":"Request could not be completed"})),
                                        ).into_response();
                                    }
                                }
                            }
                            Err(e)=>{
                                tracing::error!("Unable to deserialize current: {e}");
                                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                            }
                        }
                    }
                  }
              }
        }
        
    }else{
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"data":"Request Expired"})),
        ).into_response();
        
    }
}
