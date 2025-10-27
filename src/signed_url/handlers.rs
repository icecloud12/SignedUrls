use std::{path::PathBuf, str::FromStr};

use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use axum::{
    body::Body,
    extract::{Json, Multipart, Path, Query},
    response::IntoResponse,
};
use tokio_util::io::ReaderStream;

use crate::{
    file::model::FileDocument,
    network::{db_connection::DATABASE, DbCollection},
    project::{actions::validate_api_key, models::ProjectDocument},
    request::model::{RequestQueryParamsV2, UploadRequestDocument, ViewRequest},
    signed_url::actions::{
        save_files_to_directory, validate_signed_url, validate_signed_url_v2, ActionTypes,
    },
};
use hyper::StatusCode;
use serde_json::json;
use signed_urls::collections;
use tokio::fs::remove_file;

use super::models::DeleteFileUsingApiKey;

pub async fn process_signed_url_upload_request(
    Path(params): Path<Vec<(String, String)>>,
    multipart: Multipart,
) -> impl IntoResponse {
    //validate the url
    let collected_params = params
        .iter()
        .map(|param| param.1.clone())
        .collect::<Vec<String>>();
    let request_id: String = collected_params[0].clone();
    
    if validate_signed_url(collected_params, ActionTypes::UPLOAD_V1.to_string().as_str()).await {
        let db: &Database = DATABASE.get().unwrap();
        let request = db.collection::<UploadRequestDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": ObjectId::from_str(request_id.as_str()).unwrap()
        }, None).await.unwrap().unwrap();
        let project = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": request.project_id
        },None).await.unwrap().unwrap();
        let save_files_to_directory_result = save_files_to_directory(&request._id.to_hex(), &project._id.to_hex(), request.target, request.options, multipart, ActionTypes::UPLOAD_V1).await;
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
    tracing::info!("");
    if validate_signed_url_v2(None, query, ActionTypes::UPLOAD_V2).await{

        let db: &Database = DATABASE.get().unwrap();
        let request = db.collection::<UploadRequestDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": ObjectId::from_str(request_id.unwrap().as_str()).unwrap()
        }, None).await.unwrap().unwrap();
        let project = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{
            "_id": request.project_id
        },None).await.unwrap().unwrap();

        let save_files_to_directory_result = save_files_to_directory(&request._id.to_hex(), &project._id.to_hex(), request.target, request.options, multipart, ActionTypes::UPLOAD_V2).await;
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
        
    }else{
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"data":"Request Expired"})),
        );
        
    }
}
pub async fn process_signed_url_view_request(
    Path(params): Path<Vec<(String, String)>>,
) -> impl IntoResponse {
    let collected_params: Vec<String> = params
        .iter()
        .map(|param| param.1.clone())
        .collect::<Vec<String>>();
    let request_id: ObjectId = ObjectId::from_str(collected_params[0].as_str()).unwrap();
    let file_id = collected_params[5].clone();
    //[request_id, created, expiration, nonce, signature, file] -- tho only signature is used in valdiating signed URL
    if validate_signed_url(collected_params, ActionTypes::VIEW_V1.to_string().as_str()).await {
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
pub async fn read_file(file_id: &String, db: &Database) -> (StatusCode, Option<axum::body::Body>) {
    let file_document_result = db
        .collection::<FileDocument>(DbCollection::FILE.to_string().as_str())
        .find_one(
            doc! {
                "_id": ObjectId::from_str(&file_id.as_str()).unwrap()
            },
            None,
        )
        .await;
    match file_document_result {
        Ok(file_document_option) => match file_document_option {
            Some(file_document) => {
                let file_name_parts: Vec<&str> =
                    file_document.file_name.split(".").into_iter().collect();
                let mut extension = "";
                if file_name_parts.len() > 1 {
                    extension = file_name_parts.last().unwrap();
                }
                let file_id = file_document._id.to_hex();
                let file_name = if file_name_parts.len() == 1 {
                    &file_id
                } else {
                    &format!("{}.{}", &file_id, &extension).to_string()
                };
                let file_path = PathBuf::from(file_document.path)
                    .join(&file_id)
                    .join(file_name);
                match tokio::fs::File::open(file_path).await {
                    Ok(file) => {
                        let stream = ReaderStream::new(file);
                        let body = Body::from_stream(stream);
                        return (StatusCode::OK, Some(body));
                    }
                    Err(_) => {
                        return (StatusCode::NOT_FOUND, None);
                    }
                }
            }
            None => {
                return (StatusCode::NOT_FOUND, None);
            }
        },
        Err(_) => {
            //either by query or service unavailable
            return (StatusCode::INTERNAL_SERVER_ERROR, None);
        }
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

pub async fn delete_file_using_api_key(
    Path(params): Path<Vec<(String, String)>>,
    Json(payload): Json<DeleteFileUsingApiKey>,
) -> impl IntoResponse {
    let db = DATABASE.get().unwrap();
    if let Some(file_id) = params.get(0) {
        match ObjectId::from_str(file_id.1.clone().as_str()) {
            Ok(file_obj_id) => {
                if payload.api_key.is_some() {
                    match validate_api_key(&payload.api_key.unwrap(), None, ActionTypes::DELETE_V1)
                        .await
                    {
                        Ok(project_o) => {
                            match project_o {
                                Some(project_doc) => {
                                    let project_id = project_doc._id;

                                    let check_file_correct_project = db
                                        .collection::<FileDocument>(
                                            DbCollection::FILE.to_string().as_str(),
                                        )
                                        .find_one(
                                            doc! {
                                                collections::File::ID: file_obj_id,
                                                collections::File::PROJECT_ID: project_id,
                                            },
                                            None,
                                        )
                                        .await
                                        .unwrap();

                                    match check_file_correct_project {
                                        Some(file_doc) => {
                                            //delete db row
                                            let _ = db
                                                .collection::<FileDocument>(
                                                    DbCollection::FILE.to_string().as_str(),
                                                )
                                                .find_one_and_delete(
                                                    doc! {
                                                        "_id": file_doc._id
                                                    },
                                                    None,
                                                )
                                                .await;
                                            let file_ext =
                                                file_doc.file_name.split(".").last().unwrap();
                                            let file_id: String = file_doc._id.to_hex();
                                            let file_path = format!(
                                                "{file_path}/{file_name}",
                                                file_path = file_doc.path,
                                                file_name = format!(
                                                    "{}/{}.{}",
                                                    &file_id, &file_id, file_ext
                                                )
                                            );
                                            //delete file
                                            let file_remove_result = remove_file(file_path).await;
                                            match file_remove_result {
                                                Ok(()) => {
                                                    println!("File removed successfully");
                                                    return (
                                                        StatusCode::OK,
                                                        Json(
                                                            json!({"message": "Successfully removed file"}),
                                                        ),
                                                    )
                                                        .into_response();
                                                }
                                                Err(err) => {
                                                    println!(
                                                        "{}",
                                                        format!("Cannot remove file :{:#?}", &err)
                                                    );
                                                    return (StatusCode::BAD_REQUEST, Json(json!({"message": format!("Cannot remove file :{:#?}",&err)}))).into_response();
                                                }
                                            }
                                        }
                                        None => {
                                            return (
                                                StatusCode::BAD_REQUEST,
                                                Json(json!({"message":"Parameter mismatch"})),
                                            )
                                                .into_response()
                                        }
                                    }
                                }
                                None => {
                                    return (
                                        StatusCode::BAD_REQUEST,
                                        Json(json!({"message":"API key invalid"})),
                                    )
                                        .into_response();
                                }
                            }
                        }
                        Err(status_code) => return (status_code).into_response(),
                    }
                } else {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"message":"An unauthorized action"})),
                    )
                        .into_response();
                }
            }
            Err(_) => return (StatusCode::BAD_REQUEST).into_response(),
        }
    }

    return (
        StatusCode::BAD_REQUEST,
        Json(json!({"message":"Invalid reference for FileId"})),
    )
        .into_response();
}
