use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use mongodb::results::InsertOneResult;
use serde::Deserialize;
use axum::{extract::Json};
use serde_json::json;

use crate::actions::project;
use crate::network::Connect;
use std::any::Any;
#[derive(Deserialize)]
pub struct CreateProjectPostRequest {
    pub name: String,
}

pub async fn create_project(Json(post_request):Json<CreateProjectPostRequest>) -> impl IntoResponse{
    let db = Connect().await;
    //check if exist
    let create_request = project::insert_project_if_exists(&db, post_request.name.clone()).await;

    match create_request {
        Ok(model) => {
            let j = json!({"data":model, "message":"Project created"});
            return (StatusCode::CREATED, Json(j)).into_response();
            
        },
        Err(_) => {
            let j = json!({"data":"", "message":"Project already exists"});
            return (StatusCode::CONFLICT,Json(j)).into_response()
        }
    }
    

}

