use serde::Deserialize;
use axum::{
    extract::Json,
};

#[derive(Deserialize)]
pub struct CreateProjectPostRequest {
    pub name: String,
}
pub async fn create_project(Json(postRequest):Json<CreateProjectPostRequest>){
    println!("postRequest: {}", postRequest.name);
}
async fn create_user(Json(postRequest): Json<CreateProjectPostRequest>) {
    // ...
}

pub async fn hello(){
    println!("Hello World!222")
}
