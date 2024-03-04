use axum::{
    extract::{
        Json, Path
    }, response:: IntoResponse,
    extract::Multipart
};
use hyper::StatusCode;
use serde_json::json;

pub async fn process_signed_url_request(
    Path(params):Path<Vec<(String,String)>>,
    mut multipart: Multipart,
)-> impl IntoResponse{
    
    while let Some(mut part) = multipart.next_field().await.unwrap() {
        println!("{:#?}", part);
        let name = part.name().unwrap().to_string();
        let data = part.bytes().await.unwrap();
        
    }
    return (StatusCode::OK, Json(json!({"data":{
        "params":params,
    }})));
}