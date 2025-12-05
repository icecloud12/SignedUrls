use std::{env, str::FromStr};

use axum::{body::Body, extract::Path, response::IntoResponse, Json};
use hyper::StatusCode;
use mongodb::{bson::oid::ObjectId, Database};
use serde_json::json;
use tokio_util::io::ReaderStream;

use crate::{
    models::{file_models::FileIdUrlPair, request_models::{CreateSignedUrlViewRequest, CreateSignedUrlViewRequestV1, CreateSignedUrlViewRequestV2, ViewFileRequest, ViewFileRequestOptions, ViewRequest}}, network::{db_connection::DATABASE, DbCollection}, project::actions::{validate_api_key_v1, validate_api_key_v2}, request::actions::file_reference_is_valid, signed_url::actions::{create_hashed_signature, ActionTypes, CreateHashedSignatureResult, ViewActionTypes}};

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
    let public_key_o = public_key;
    let mut signatures = Vec::new();
    let file_id_collection = files.unwrap();
    match public_key_o {
        Some(public_key) => {
            match version {
                ViewActionTypes::V1 => {
                   match validate_api_key_v1(db, &public_key).await {
                        Ok(project_o)=>{
                            match project_o {
                                Some(project) => {
                                    let created_hashed_signature: CreateHashedSignatureResult =
                                        create_hashed_signature(
                                            &project._id.to_hex(),
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
                                    let t_sig = signatures.get(0).unwrap();
                                    let doc = ViewRequest {
                                        project_id: project._id,
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
                                None => {
                                    tracing::warn!("project with public key [{}] does not match", public_key);
                                    return (StatusCode::BAD_REQUEST).into_response()}
                            }
                        }
                        Err(validate_error)=>{
                            tracing::error!("{:#?}",validate_error);
                            return (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                        }
                   } 
                }
                ViewActionTypes::V2 => {
                     match validate_api_key_v2(&db, &public_key, secret_key.clone()).await{
                         Ok(bucket_document_o)=>{
                             match bucket_document_o {
                                 None => {
                                     tracing::warn!("No results found for public_key [{}]", public_key);
                                     return (StatusCode::BAD_REQUEST).into_response();
                                 }
                                 Some (bucket)=>{
                                    file_id_collection
                                        .iter()
                                        .for_each(|file_id| {
                                            let created_hashed_signature: CreateHashedSignatureResult =
                                                create_hashed_signature(
                                                    &bucket._id.to_hex(),
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
                                            signatures.push(created_hashed_signature);
                                    });
                                    let t_sig = signatures.get(0).unwrap();
                                    let doc = ViewRequest {
                                        project_id: bucket._id,
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
                                                                Some(option_val) => Some(option_val),
                                                                None => Some(false)
                                                            }
                                                        },
                                                        is_consumed: Some(false)
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
                         Err(bucket_validate_err)=>{
                             tracing::error!("{:#?}",bucket_validate_err);
                             return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                         }
                     }                   
                }
            };
        }
        None=>{
           return (StatusCode::UNAUTHORIZED).into_response();
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
