use std::{path::PathBuf, str::FromStr};

use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use axum::{
    body::Body,
    extract::{Json, Path},
    response::IntoResponse,
};
use tokio_util::io::ReaderStream;

use crate::{
    models::{file_models::FileDocument, signed_url_models::DeleteFileUsingApiKey},
    network::{db_connection::DATABASE, DbCollection},
    project::actions::validate_api_key_v2,
    signed_url::actions::ActionTypes,
};
use hyper::StatusCode;
use serde_json::json;
use signed_urls::collections;
use tokio::fs::remove_file;

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

pub async fn delete_file_using_api_key(
    Path(params): Path<Vec<(String, String)>>,
    Json(payload): Json<DeleteFileUsingApiKey>,
) -> impl IntoResponse {
    let db = DATABASE.get().unwrap();
    if let Some(file_id) = params.get(0) {
        match ObjectId::from_str(file_id.1.clone().as_str()) {
            Ok(file_obj_id) => {
                if payload.api_key.is_some() {
                    match validate_api_key_v2(&db, &payload.api_key.unwrap(), None).await {
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
