use axum::{
    response:: IntoResponse,
    extract::{
        Path,
        Json,
    }
};
use hyper::StatusCode;
use serde_json::json;
use super::models::ExtractSignedUrlParametersRequest;

pub async fn process_signed_url_request(
    Path(params):Path<Vec<(String,String)>>,
    Json(body): Json<ExtractSignedUrlParametersRequest>
)-> impl IntoResponse{
    
    return (StatusCode::OK, Json(json!({"data":{
        "params":params,
        "body": body
    }})));
}