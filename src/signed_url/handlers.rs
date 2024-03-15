use std::{path::PathBuf, str::FromStr};

use mongodb::{bson::{doc, oid::ObjectId}, Database};

use axum::{
    body::Body, extract::{
        Json, Multipart, Path
    }, response:: IntoResponse
};
use tokio_util::io::ReaderStream;

use crate::{file::{self, model::FileDocument}, network::{db_connection::DATABASE, DbCollection}, request::model::ViewRequest, signed_url::actions::{save_files_to_directory, validate_signed_url}};
use hyper::StatusCode;
use serde_json::json;
use tokio::fs::File;

pub async fn process_signed_url_upload_request(
    Path(params):Path<Vec<(String,String)>>,
    multipart: Multipart,
)-> impl IntoResponse{
    //validate the url
    let collected_params = params.iter().map(|param| param.1.clone()).collect::<Vec<String>>();
    let request_id: String = collected_params[0].clone();

    if validate_signed_url(collected_params, "upload").await{
        let save_files_to_directory_result  = save_files_to_directory(request_id,multipart).await;
        match save_files_to_directory_result {
            Ok(res) => {
                return (StatusCode::OK,Json(json!({"data":res})));
            },
            Err(_)=>{
                return (StatusCode::BAD_REQUEST,Json(json!({"data":"Request could not be completed"})));
            }
        }
        
    }
    return (StatusCode::BAD_REQUEST, Json(json!({"data":"Request Expired"})))   
}
pub async fn process_signed_url_view_request(
    Path(params): Path<Vec<(String,String)>>
) -> impl IntoResponse {
    let collected_params: Vec<String> = params.iter().map(|param| param.1.clone()).collect::<Vec<String>>();
    let request_id: ObjectId = ObjectId::from_str(collected_params[0].as_str()).unwrap();
    let file_id = collected_params[5].clone();
    //[request_id, created, expiration, nonce, signature, file] -- tho only signature is used in valdiating signed URL
    if validate_signed_url(collected_params, "view").await {
        //if validated check if request view document has file
        let db:&Database = DATABASE.get().unwrap();
        let view_request_document_result = db.collection::<ViewRequest>(DbCollection::REQUEST.to_string().as_str()).find_one(doc!{ "_id": request_id}, None).await.unwrap().unwrap(); // we shouldn't technically touch the database directly so we can simply assume thigns would work out
        if view_request_document_result.files.contains(&file_id){
            println!("files in correct relative to request");
            //file_id is in the list. check if it is a valid file_id referenec
            let file_document_result = 
            db.collection::<FileDocument>(DbCollection::FILE.to_string().as_str()).find_one(doc!{"_id" : ObjectId::from_str(file_id.as_str()).unwrap()}, None).await.unwrap();

            match  file_document_result{
                Some(file_document)=>{
                    println!("some file");
                    let file_extention:String = file_document.file_name.split(".").last().unwrap().to_string();
                    let file_id:String = file_document._id.to_hex();
                    let file_name = format!("{}.{}",&file_id,&file_extention);
                    let file_path = PathBuf::from(file_document.path).join(format!("{}/{}",file_id,file_name));
                    println!("{}",file_path.to_str().unwrap());
                    match tokio::fs::File::open(file_path).await {
                        Ok(file) => {
                            
                            let stream = ReaderStream::new(file);
                            let body = Body::from_stream(stream);
                            return (StatusCode::OK, body).into_response()
                        },
                        Err(_) => return (StatusCode::NOT_FOUND).into_response()
                    }
                },
                None => return (StatusCode::NOT_FOUND, "File not found").into_response()
            }
        }else{
            return (StatusCode::UNAUTHORIZED, "Unauthorized file acces").into_response()
        };
    }
    else{
        return (StatusCode::BAD_REQUEST).into_response()
    }
}