

use std::str::FromStr;

use hyper::StatusCode;
use serde_json::json;
use axum::{
    body::Body, extract::{Json, Path}, response:: IntoResponse
};
use mongodb::{bson::oid::ObjectId, Database};
use crate::network::DbCollection;
use crate::network::db_connection::DATABASE;
use crate::project::actions::get_project_id_by_name;
use crate::signed_url::actions::{
    CreateHashedSignatureResult,
    create_hashed_signature
};
use tokio_util::io::ReaderStream;
use super::model::{CreateSignaturePostRequestOptions, CreateSignedUrlPostRequest, UploadRequest};
use super::actions::file_reference_is_valid;

pub async fn create_upload_request(Json(post_request):Json<CreateSignedUrlPostRequest>) -> impl IntoResponse{
    let db: &Database = DATABASE.get().unwrap();
    let CreateSignedUrlPostRequest{
        project_name, 
        duration, 
        is_consumable: _,
        target ,
        is_public
    }
     = post_request;
    let permission = "upload".to_string();
    if project_name.is_some() {
        //let project_name = post_request.project_name.unwrap(); 
        let project_id_fetch = get_project_id_by_name(project_name.clone().unwrap()).await;

        if project_id_fetch.is_some() {

            let address=std::env::var("ADDRESS").unwrap();
            let port = std::env::var("PORT").unwrap();

            let created_hashed_signature:CreateHashedSignatureResult = create_hashed_signature(
                &project_id_fetch.unwrap(), 
                &duration.unwrap_or_else(|| std::env::var("DEFAULT_DURATION_AS_SECONDS").unwrap().to_string().parse::<u64>().unwrap()),
                &permission.clone());
            
            let doc: UploadRequest = UploadRequest {
                project_name: project_name.unwrap(), //we use the id when we can find the name
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

            
            let insert_request_id = &db.collection::<UploadRequest>(DbCollection::REQUEST.to_string().as_str()).insert_one(doc, None).await.unwrap().inserted_id.as_object_id().unwrap().to_string();
        
            let generated_url:String = format!("{}:{}/id/{}/permission/{}/created/{}/expiration/{}/nonce/{}/signature/{}",address, port, insert_request_id, permission,created_hashed_signature.date_created,created_hashed_signature.expiration_date,created_hashed_signature.nonce,created_hashed_signature.hashed_signature_base_64);
            return (StatusCode::CREATED, Json(json!(
                {"data":{
                    "request_id": insert_request_id,
                    "url": generated_url
                }
            })));
        }
    }
    return (StatusCode::BAD_REQUEST, Json(json!(
        {"data":None::<String>, "message":"Failed to create signed-url"}
    )));

}

pub async fn process_preview_request( Path(params): Path<Vec<(String, String)>>) -> impl IntoResponse{
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
                        let path = initial_path.join(format!("{}.{}",file_document._id.to_hex(), file_ext));            
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