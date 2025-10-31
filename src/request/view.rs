use std::{env, str::FromStr};

use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use mongodb::{bson::oid::ObjectId, Database};
use serde_json::json;

use crate::{file::model::FileIdUrlPair, network::{db_connection::DATABASE, DbCollection}, project::actions::validate_api_key, request::model::{CreateSignedUrlViewRequest, CreateSignedUrlViewRequestV1, CreateSignedUrlViewRequestV2, ViewFileRequest, ViewFileRequestOptions, ViewRequest}, signed_url::actions::{create_hashed_signature, ActionTypes, CreateHashedSignatureResult, ViewActionTypes}};

async fn create_view_request(
    version: ViewActionTypes,
    post_request: CreateSignedUrlViewRequest,
) -> impl IntoResponse {

    let db: &Database = DATABASE.get().unwrap();
    let CreateSignedUrlViewRequest {
        duration,
        files,
        public_key,
        secret_key,
        is_consumable
    } = post_request;
    match public_key{
        Some(public_key) => match validate_api_key(&db, &public_key, secret_key, ActionTypes::from(version)).await {
            Ok(bucket_document)=>{
                match bucket_document {
                    
                    Some(project_doc) => {
                        let file_id_collection = files.unwrap();

                        let project_id = project_doc._id;
                        let mut signatures = Vec::new();

                        match version {
                            ViewActionTypes::V1 => {
                                let created_hashed_signature: CreateHashedSignatureResult =
                                    create_hashed_signature(
                                        &project_id.to_hex(),
                                        &duration.unwrap_or_else(|| {
                                            std::env::var("DEFAULT_DURATION_AS_SECONDS")
                                                .unwrap()
                                                .to_string()
                                                .parse::<u64>()
                                                .unwrap()
                                        }),
                                        &version.to_string(),
                                        None
                                    );
                                signatures.push(created_hashed_signature);
                            }
                            ViewActionTypes::V2 => {
                                file_id_collection
                                    .iter()
                                    .for_each(|file_id| {
                                        let created_hashed_signature: CreateHashedSignatureResult =
                                            create_hashed_signature(
                                                &project_id.to_hex(),
                                                &duration.unwrap_or_else(|| {
                                                    std::env::var("DEFAULT_DURATION_AS_SECONDS")
                                                        .unwrap()
                                                        .to_string()
                                                        .parse::<u64>()
                                                        .unwrap()
                                                }),
                                                &version.to_string(),
                                                Some(file_id)
                                            );
                                        println!("pushing signature");

                                        signatures.push(created_hashed_signature);
                                    });
                            }
                        };
                        let t_sig = signatures.get(0).unwrap();
                        let doc = ViewRequest {
                            project_id: project_id,
                            date_created: t_sig.date_created,
                            expiration_date: t_sig.expiration_date,
                            permission: version.to_string(),
                            files: file_id_collection.clone(),
                            options: None,
                        };

                        let insert_request_id = &db
                            .collection::<ViewRequest>(DbCollection::REQUEST.to_string().as_str())
                            .insert_one(doc, None)
                            .await
                            .unwrap()
                            .inserted_id
                            .as_object_id()
                            .unwrap();
                        let prefix = env::var("PREFIX").unwrap();
                        let replaced_url = env::var("REPLACED_URL").unwrap();

                        match version {
                            ViewActionTypes::V1 => {
                                let signature = signatures.get(0).unwrap();
                                let generated_url:String = format!("https://{}{}/id/{}/permission/{}/created/{}/expiration/{}/nonce/{}/signature/{}/file/",
                                    replaced_url,
                                    prefix,
                                    insert_request_id,
                                    version.to_string(),
                                    signature.date_created,
                                    signature.expiration_date,
                                    signature.nonce,
                                    signature.hashed_signature_base_64);
                                return (
                                    StatusCode::CREATED,
                                    Json(json!(
                                        {"data":{
                                            "request_id": insert_request_id,
                                            "base_url": generated_url
                                        }
                                    })),
                                ).into_response();
                            }
                            ViewActionTypes::V2 => {
                                let mut file_url_pairs: Vec<FileIdUrlPair> = Vec::new();
                                let mut view_file_requests: Vec<ViewFileRequest> = Vec::new();
                                signatures.iter().enumerate().for_each(|(index, signature)| {
                                    let file_id = file_id_collection.get(index).unwrap();
                                    let file_object_id = ObjectId::from_str(&file_id);
                                    match file_object_id {
                                        Ok(file_object_id)=>{
                                            file_url_pairs.push(FileIdUrlPair {
                                                id: file_id_collection.get(index).unwrap().to_string(),
                                                url:  Some(format!("https://{origin}{prefix}/v2/file/{file_id}?r={insert_request_id}&c={date_created}&e={expiration}&n={nonce}&s={signature}",
                                                    origin = replaced_url ,
                                                    file_id= file_id,
                                                    date_created = signature.date_created,
                                                    expiration = signature.expiration_date,
                                                    nonce = signature.nonce,
                                                    signature = signature.hashed_signature_base_64).to_string()),
                                                error: None
                                            });
                                    
                                            view_file_requests.push(ViewFileRequest {
                                                request_id: *insert_request_id,
                                                file_id: file_object_id,
                                                options: Some(ViewFileRequestOptions{
                                                    is_consumable: {
                                                        match is_consumable{
                                                            Some(option_val) => {option_val},
                                                            None => false
                                                        }
                                                    },
                                                    is_consumed: false
                                                })
                                            });
                                        }
                                        Err(_)=>{

                                            file_url_pairs.push(FileIdUrlPair {
                                                id: file_id_collection.get(index).unwrap().to_string(),
                                                url: None,
                                                error: Some(String::from("Invalid file reference format"))
                                            });
                                        }
                                    }

                                });
                                let _ = db.collection::<ViewFileRequest>(DbCollection::VIEW_FILE_REQUEST.to_string().as_str()).insert_many(view_file_requests, None).await;
                                return (
                                    StatusCode::CREATED,
                                    Json(json!({
                                        "request_id": insert_request_id,
                                        "files": file_url_pairs
                                    })),
                                ).into_response();
                            }
                        }
                    }
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                        ).into_response();
                    }    
                }
            }
            Err(status_code)=>{
                return (status_code).into_response();
            }
            
        },
        None => {
            return (
                StatusCode::UNAUTHORIZED,
            ).into_response()
        }
    }
}
pub async fn create_view_request_v1(
    Json(post_request): Json<CreateSignedUrlViewRequestV1>,
) -> impl IntoResponse {
    match CreateSignedUrlViewRequest::try_from(post_request){
        Ok(request) => {
            return create_view_request(ActionTypes::VIEW_V1.try_into().unwrap(), request).await.into_response();
        }
        Err(e)=>{ return (e).into_response(); }
    }
}
pub async fn create_view_request_v2(
    Json(post_request): Json<CreateSignedUrlViewRequestV2>,
) -> impl IntoResponse {
    match CreateSignedUrlViewRequestV2::try_into(post_request){
        Ok(request) =>{
            return create_view_request(ActionTypes::VIEW_V2.try_into().unwrap(), request).await.into_response();
        }
        Err(e)=>{ return (e).into_response(); }
    }
}
