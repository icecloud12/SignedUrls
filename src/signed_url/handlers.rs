

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