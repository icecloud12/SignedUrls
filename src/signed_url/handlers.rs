use std::{alloc::System, f64::consts::E, path::PathBuf, str::FromStr, time::{SystemTime, UNIX_EPOCH}};

use mongodb::{bson::{doc, oid::ObjectId}, results::InsertOneResult, Database};

use axum::{
    body::{Body, Bytes}, extract::{
        Json, Multipart, Path
    }, response::{ IntoResponse, Response}
};
use tokio_util::io::ReaderStream;
use crate::{file::model::{FileDocumentInsertRow, FileDocumentOptions}, network::DbCollection, request::model::{CreateSignaturePostRequestOptions, DirectUploadRequest}, signed_url::actions::save_file_to_directory};

use crate::{file::{self, model::FileDocument}, network::{db_connection::DATABASE}, project::actions::validate_api_key, request::model::{ViewRequest}, signed_url::actions::{save_files_to_directory, validate_signed_url}};
use hyper::{HeaderMap, StatusCode};
use serde_json::json;
use tokio::fs::{self, File};

use super::actions::{direct_upload_extract_multipart, multipartFile, ActionTypes};

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
                    
                    let file_extention:String = file_document.file_name.split(".").last().unwrap().to_string();
                    let file_id:String = file_document._id.to_hex();
                    let file_name = format!("{}.{}",&file_id,&file_extention);
                    let file_path = PathBuf::from(file_document.path).join(format!("{}/{}",file_id,file_name));
                    
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
pub async fn direct_upload(headermap:HeaderMap, multipart: Multipart) -> impl IntoResponse{
    let mut insert_into_file_directory_result:Vec<(bool,Result<String,String>)> = Vec::new();
    let response = match validate_api_key(headermap).await {
        Some(project_document) => {
            match direct_upload_extract_multipart(multipart).await{
                Ok((multi_part_files, target, is_public,is_consumable, max_file_size))=>{
                    
                    //create a request of type direct-upload
                    let db = DATABASE.get().unwrap();
                    let doc = DirectUploadRequest {
                        project_id : project_document._id.to_hex(),
                        date_created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        permission: ActionTypes::DIRECT_UPLOAD.to_string(),
                        target: target.clone(),
                        options: Some(
                            CreateSignaturePostRequestOptions {
                                is_consumable: Some(is_consumable),
                                is_consumed: Some(true),
                                is_public: Some(is_public), //get data from the multipart
                            }
                        )
                    };
                    let collection = db.collection::<DirectUploadRequest>(
                        DbCollection::REQUEST.to_string().as_str());
                    collection.insert_one(doc, None).await.unwrap();
                    let project_id = project_document._id.to_hex();
                    let initial_path = std::path::PathBuf::from("./data/").join(format!("{}/", project_id)).join(target);
                    let mut file_saved:Vec<super::models::File> = Vec::new();
                    
                    //insert all images to directory
                    for mulitipart_file in multi_part_files {
                        if mulitipart_file.file_size_is_valid {
                            let file_document_insert = FileDocumentInsertRow {
                                file_name : mulitipart_file.file_name.clone(),
                                path: initial_path.to_str().unwrap().to_string(),
                                options: FileDocumentOptions {
                                    is_public: is_public
                                }
                            };
                            let insert_file_insert_result: InsertOneResult = db.collection::<FileDocumentInsertRow>(DbCollection::FILE.to_string().as_str()).insert_one(file_document_insert, None).await.unwrap();
    
                            let new_file_name = insert_file_insert_result.inserted_id.as_object_id().unwrap().to_string();
                            let new_file_directory:PathBuf = initial_path.join(format!("{}/", new_file_name));
                            let create_file_directory_result = if !(std::fs::metadata(&new_file_directory).is_ok() && std::fs::metadata(&new_file_directory).expect("").is_dir()){
                                match std::fs::create_dir_all(&new_file_directory) {
                                    Ok(_)=>{
                                        Ok(())
                                        
                                    }
                                    Err(_)=> {
                                        Err(format!("Cannot create this path: {:#?}",new_file_directory.to_str().unwrap()))
                                    
                                    }
                                }
                            }else{ //path is ok and is dir
                                Ok(())
                            };
                            match create_file_directory_result {
                                Ok(_)=>{
                                    
                                    let file_extention = mulitipart_file.file_name.split(".").last().unwrap();
                                    let new_file_path: String = new_file_directory.clone().join(format!("{}.{}",new_file_name,file_extention)).to_str().unwrap().to_string();
                                    
                                    let res = save_file_to_directory(mulitipart_file.file_name, new_file_name, new_file_path, mulitipart_file.bytes).await;
                                    insert_into_file_directory_result.push((true, Ok(res._id.clone())));
                                    file_saved.push(res);
                                    
                                },
                                Err(_)=>{
                                    insert_into_file_directory_result.push(
                                        (
                                            false,
                                            Err(
                                                format!("Cannot create this path: {:#?}",new_file_directory.to_str().unwrap()
                                                )
                                            )
                                        )
                                    );                                
                                }
                            }
                        }else{
                            insert_into_file_directory_result.push(
                                (
                                    false,
                                    Err("File size limit exceeded".to_string())
                                    
                                )
                            );       
                        }
                        
                    }

                }
                Err(err_string)=>{
                    return (StatusCode::BAD_REQUEST, err_string).into_response()
                }
            }
            
            (StatusCode::OK, Json(insert_into_file_directory_result)).into_response()
        },
        None => {
            (StatusCode::BAD_REQUEST, Json(json!(
                {"data":None::<String>, "message": "this request is invalid/unauthorized"}
            ))).into_response()
        }
    };
    return response;
}