use std::{path::PathBuf, str::FromStr};

use axum::{extract::{Multipart, Path, Query}, response::IntoResponse, Json};
use hyper::StatusCode;
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use serde_json::json;
use signed_urls::collections;
use tokio::{fs::{remove_file, File}, io::AsyncWriteExt};
use crate::{
    models::{file_models::{FileDocumentInsertRow, FileDocumentOptions}, project_models::{BucketDocument, ProjectDocument}, request_models::{MergeRequestOptions, RequestQueryParamsV2, UploadRequestDocument, UploadRequestWithBucketDocument}}, network::{db_connection::DATABASE, DbCollection}, signed_url::actions::{save_files_to_directory, validate_signed_url_v1, validate_signed_url_v2, ActionTypes, UploadActionTypes}};

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
        let request = db.collection::<UploadRequestDocument>(DbCollection::REQUEST.to_string().as_str()).find_one(doc!{
            collections::Request::ID : ObjectId::from_str(request_id.as_str()).unwrap()
        }, None).await.unwrap().unwrap();
        let project = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            collections::Bucket::ID : request.project_id
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
    mut multipart: Multipart,
)-> impl IntoResponse{
    let query = query.0;
    let request_id = query.request.to_owned().unwrap();
    if validate_signed_url_v2(None, &query, ActionTypes::UPLOAD_V2).await{

        let db: &Database = DATABASE.get().unwrap();
        let aggregate_pipeline = vec![
            doc!{ "$match": {
              "_id": ObjectId::from_str(&request_id).unwrap()
            }},
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
                                tracing::info!("{:#?}", &request_id);
                                let UploadRequestWithBucketDocument { _id: request_id, target, options, bucket: BucketDocument{ _id: bucket_id, .. }, .. } = document;
                                //check for file size
                                let mut initial_path: std::path::PathBuf = std::path::PathBuf::from("./data/")
                                    .join(format!("{}/", &bucket_id.to_hex().clone()));
                                if let Some(target) = target{        
                                    initial_path = initial_path.join(format!("{}/", target));
                                }
                                let merge_options = match options {
                                    None => {MergeRequestOptions::default()}
                                    Some(r_options) => {
                                        MergeRequestOptions::from(r_options)
                                    }
                                };


                                while let Some(mut part) = multipart.next_field().await.unwrap(){
                                    if part.name().unwrap_or_else(|| "") == "file"{
                                        match part.file_name() {
                                            Some(part_file_name) => {
                                                let file_name = part_file_name.to_string();                                 
                                                let file_document_insert: FileDocumentInsertRow = FileDocumentInsertRow {
                                                    file_name: file_name.clone(),
                                                    path: initial_path.to_str().unwrap().to_string(),
                                                    options: FileDocumentOptions {
                                                        is_public: merge_options.is_public,
                                                    },
                                                    project_id: bucket_id,
                                                    request_id: request_id,
                                                };
                                                let db: &Database =  DATABASE.get().unwrap();
                                                let insert_file_insert_result = db
                                                    .collection::<FileDocumentInsertRow>(
                                                        DbCollection::FILE.to_string().as_str(),
                                                    )
                                                    .insert_one(file_document_insert, None)
                                                    .await
                                                    .unwrap();
                                                let new_file_name = insert_file_insert_result
                                                    .inserted_id
                                                    .as_object_id()
                                                    .unwrap()
                                                    .to_string();

                                                let new_file_directory: PathBuf =
                                                    initial_path.join(format!("{}/", new_file_name));
                                                if !( tokio::fs::metadata(&new_file_directory).await.is_ok()
                                                    && tokio::fs::metadata(&new_file_directory).await.expect("").is_dir()) {

                                                    match std::fs::create_dir_all(&new_file_directory) {
                                                        Ok(_a) => {
                                                            //do something on dir creation
                                                            let file_extention = new_file_name.split(".").last().unwrap();
                                                            let new_file_path: String = new_file_directory
                                                                .clone()
                                                                .join(format!("{}.{}", new_file_name, file_extention))
                                                                .to_str()
                                                                .unwrap()
                                                                .to_string();
                                                            
                                                            let size_limit = merge_options.size_limit; 
                                                            let mut total_chunks: usize = 0;
                                                            let mut file= tokio::fs::File::create(new_file_path.clone()).await.unwrap();
                                                            while let Some(streamed_chunk) = &part.chunk().await.unwrap(){
                                                                let _ = file.write_all(&streamed_chunk).await;
                                                                total_chunks += streamed_chunk.len();
                                                                if size_limit.is_some(){
                                                                    if total_chunks > size_limit.unwrap() {
                                                                        tracing::warn!("File Size Limit Exceeded");
                                                                        if remove_file(new_file_path).await.is_err() {
                                                                            tracing::error!("Something went wrong when trying to remove file {new_file_name}")
                                                                        }
                                                                        return (StatusCode::PAYLOAD_TOO_LARGE).into_response(); 
                                                                    }
                                                                }
                                                            }
                                                            return (StatusCode::CREATED, Json(json!({"request_id": request_id.to_string(), "file_id": new_file_name}))).into_response();
                                                            // let saved_file = save_file_to_directory(
                                                            //     original_file_name,
                                                            //     new_file_name,
                                                            //     new_file_path,
                                                            //     chunks,
                                                            // )
                                                            // .await;
                                                            // created_files.push(saved_file);
                                                        }
                                                        Err(_) => {
                                                            tracing::error!("cannot create in this directory");
                                                            return (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                                                        }
                                                    }
                                                }
                                            }
                                            None => {
                                                return (StatusCode::BAD_REQUEST).into_response()
                                            }
                                        }
                                    }
                                }
                                //missing file key
                                return (StatusCode::BAD_REQUEST, Json(json!({"message": "Missing multipart key `file`"}))).into_response()
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
