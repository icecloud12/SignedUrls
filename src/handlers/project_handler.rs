use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

use mongodb::Database;
use serde::Deserialize;
use axum::{extract::Json};
use serde_json::json;

use crate::actions::project;
use crate::network;
#[derive(Deserialize)]
pub struct CreateProjectPostRequest {
    pub name: String,
}

pub async fn create_project(Json(post_request):Json<CreateProjectPostRequest>) -> impl IntoResponse{
    let db = network::Db_Connection::DATABASE.get().unwrap();
    //check if exist
    let create_request = project::insert_project_if_exists( &post_request.name).await;
    match create_request {
        Some(request) => {
            //then create directory
            project::create_project_directory(&request).await;
            let j = json!({"data":{"id":request, "project_name": post_request.name.as_str()}, "message":"success"});

            return (StatusCode::CREATED, Json(j)).into_response();
        },
        None => {
            let j = json!({"data":"", "message":"Project already exists"});
            return (StatusCode::CONFLICT,Json(j)).into_response();
        }
    };   
}

