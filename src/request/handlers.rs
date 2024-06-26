

use std::{env, str::FromStr};

use hyper::{HeaderMap, StatusCode};
use serde_json::json;
use axum::{
    body::Body, extract::{Json, Path}, response:: IntoResponse
};
use mongodb::{bson::oid::ObjectId, Database};
use crate::{network::DbCollection, signed_url::actions::ActionTypes};
use crate::network::db_connection::DATABASE;
use crate::project::actions::{
    validate_api_key
};
use crate::signed_url::actions::{
    CreateHashedSignatureResult,
    create_hashed_signature
};
use tokio_util::io::ReaderStream;
use super::model::{
    //post_request
    CreateSignaturePostRequestOptions, CreateSignedUrlPostRequest, CreateSignedUrlViewRequest, UploadRequest, ViewRequest
};
use super::actions::file_reference_is_valid;

pub async fn create_upload_request(Json(post_request):Json<CreateSignedUrlPostRequest>) -> impl IntoResponse{

    let CreateSignedUrlPostRequest{
        duration, 
        is_consumable: _,
        target ,
        is_public,
        api_key
    }
    = post_request;
    if api_key.is_some(){
        match validate_api_key(api_key.unwrap()).await{
            Some(project_doc)=>{
                
                let permission = ActionTypes::UPLOAD.to_string();
                let project_id = project_doc._id.to_hex();
                let address=std::env::var("ADDRESS").unwrap();
                let port = std::env::var("PORT").unwrap();
                let created_hashed_signature:CreateHashedSignatureResult = create_hashed_signature(
                    &project_id.clone(), 
                    &duration.unwrap_or_else(|| std::env::var("DEFAULT_DURATION_AS_SECONDS").unwrap().to_string().parse::<u64>().unwrap()),
                    &permission.clone()
                );
                let doc: UploadRequest = UploadRequest {
                    project_id: project_id,
                    date_created: created_hashed_signature.date_created.clone(),
                    expiration_date: created_hashed_signature.expiration_date.clone(),
                    options:  CreateSignaturePostRequestOptions {
                        is_consumable: post_request.is_consumable.clone(),
                        is_consumed: Some(false),
                        is_public: Some(is_public).unwrap_or_else(|| Some(false))
                    },
                    permission: permission.clone(),
                    target: target.unwrap()
                };
                let db: &Database = DATABASE.get().unwrap();
                let insert_request_id = &db.collection::<UploadRequest>(DbCollection::REQUEST.to_string().as_str()).insert_one(doc, None).await.unwrap().inserted_id.as_object_id().unwrap().to_string();
                let prefix = env::var("PREFIX").unwrap();
                let replaced_url = env::var("REPLACED_URL").unwrap();
                let generated_url:String = format!("https://{}{}/id/{}/permission/{}/created/{}/expiration/{}/nonce/{}/signature/{}",replaced_url, prefix, insert_request_id, permission,created_hashed_signature.date_created,created_hashed_signature.expiration_date,created_hashed_signature.nonce,created_hashed_signature.hashed_signature_base_64);
                return (StatusCode::CREATED, Json(json!(
                    {"data":{
                        "request_id": insert_request_id,
                        "url": generated_url
                    }
                })));
            }
            None=>{
                return (StatusCode::BAD_REQUEST, Json(json!(
                    {"data":None::<String>, "message":"Failed to create signed-url"}
                )));
            }
        }
        
    }
    else{
        return (StatusCode::UNAUTHORIZED,  Json(json!(
            {"data":None::<String>, "message":"Failed to authorize"}
        )));
    }
   
    

}

pub async fn process_public_read_access( Path(params): Path<Vec<(String, String)>>) -> impl IntoResponse{
    let file_id = params[0].1.clone();
    //check if file is public
    let file_object_id = ObjectId::from_str(&file_id);
    if file_object_id.is_err(){
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error")));
    }else{
        match file_reference_is_valid(file_object_id.unwrap()).await{
            Ok(file_document)=>{
                if file_document.options.is_public{

                        let initial_path = std::path::PathBuf::from(file_document.path.clone());
                        let file_ext = file_document.file_name.split(".").last().unwrap();
                        let file_document_id = file_document._id.to_hex();
                        let path = initial_path.join(format!("{}/{}.{}",file_document_id,file_document_id, file_ext));            
                        println!("filePath: {}",&path.as_os_str().to_str().unwrap().to_string());
                        let file = match tokio::fs::File::open(path).await {
                            Ok(file) => file,
                            Err(err) => return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err))),
                        };
                        // convert the `AsyncRead` into a `Stream`
                        let stream = ReaderStream::new(file);
                        // convert the `Stream` into an `axum::body::HttpBody`
                        let body = Body::from_stream(stream);
                        return Ok((StatusCode::OK,body).into_response());
                    }else{
                        return Err((StatusCode::NOT_FOUND, format!("File not found")))
                    }
            },
            Err(response)=>  return Err(response)
        }
    }
}
pub async fn create_view_request(Json(post_request): Json<CreateSignedUrlViewRequest>) -> impl IntoResponse {
    
    let CreateSignedUrlViewRequest {
        duration,
        file_id_collection,
        api_key
    } = post_request;
    if api_key.is_some(){
        match validate_api_key(api_key.unwrap()).await {
            Some(project_doc)=>{
                
                let permission = ActionTypes::VIEW.to_string();
                let project_id = project_doc._id.to_hex();
                let address=std::env::var("ADDRESS").unwrap();
                let port = std::env::var("PORT").unwrap();
                let created_hashed_signature:CreateHashedSignatureResult = create_hashed_signature(
                    &project_id.clone(), 
                    &duration.unwrap_or_else(|| std::env::var("DEFAULT_DURATION_AS_SECONDS").unwrap().to_string().parse::<u64>().unwrap()),
                    &permission.clone()
                );
                let doc: ViewRequest = ViewRequest{
                    project_id: project_id,
                    date_created: created_hashed_signature.date_created,
                    expiration_date: created_hashed_signature.expiration_date,
                    permission: permission.clone(),
                    files: file_id_collection.unwrap(),
                    options:None
                };
    
                let db:&Database = DATABASE.get().unwrap();
                let insert_request_id = &db.collection::<ViewRequest>(DbCollection::REQUEST.to_string().as_str()).insert_one(doc, None).await.unwrap().inserted_id.as_object_id().unwrap().to_string();
                let prefix = env::var("PREFIX").unwrap();
                let replaced_url = env::var("REPLACED_URL").unwrap();
                let generated_url:String = format!("https://{}{}/id/{}/permission/{}/created/{}/expiration/{}/nonce/{}/signature/{}/file/",replaced_url , prefix, insert_request_id, permission,created_hashed_signature.date_created,created_hashed_signature.expiration_date,created_hashed_signature.nonce,created_hashed_signature.hashed_signature_base_64);
                return (StatusCode::CREATED, Json(json!(
                    {"data":{
                        "request_id": insert_request_id,
                        "base_url": generated_url
                    }
                })));
    
            },
            None => {
                return (StatusCode::BAD_REQUEST, Json(json!(
                    {"data":None::<String>, "message":"Failed to create signed-url"}
                )));
            }
        }   
    }else{
        return (StatusCode::UNAUTHORIZED, Json(json!(
            {"data":None::<String>, "message":"Failed to authorize"}
        )))
    }

}

