

use axum::{
    extract::{
        Json, Multipart, Path
    }, response:: IntoResponse
};
use crate::signed_url::actions::{save_files_to_directory, validate_signed_url};

use hyper::StatusCode;
use serde_json::json;

pub async fn process_signed_url_upload_request(
    Path(params):Path<Vec<(String,String)>>,
    mut multipart: Multipart,
)-> impl IntoResponse{
    //validate the url
    
    match validate_signed_url(params, multipart, "upload").await {
        Ok(validatedResult) =>{
            let _ = save_files_to_directory(validatedResult).await;
            return (StatusCode::OK,Json(json!({"data":"Operation Successful"})));
        }
        Err(e) => {return (StatusCode::BAD_REQUEST, Json(json!({"data":"Request Expired"})))}
    };
    
    
}