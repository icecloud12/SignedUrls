use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use hyper::StatusCode;
use rand::{self, Rng};
use serde_json::json;
use sha3::{Digest, Sha3_256};
use axum::{
    response:: IntoResponse,
    extract::Json
};
use mongodb::Database;
use crate::network::DB_Collection;
use crate::network::Db_Connection::DATABASE;
use crate::actions::{project_actions, signed_url_actions::{
    CreateHashedSignatureResult,
    create_hashed_signature
}};
use crate::models::RequestModel::{CreateSignedUrlPostRequest, InsertRequest, CreateSignaturePostRequestOptions};



pub async fn create_request(Json(post_request):Json<CreateSignedUrlPostRequest>) -> impl IntoResponse{
    let db: &Database = DATABASE.get().unwrap();
    let CreateSignedUrlPostRequest{
        project_name, 
        duration, 
        is_consumable,
        permission
    }
     = post_request;
    if project_name.is_some() {
        //let project_name = post_request.project_name.unwrap(); 

        let project_id_fetch = project_actions::get_project_id_by_name(project_name.clone().unwrap()).await;

        if project_id_fetch.is_some() {

            let address=std::env::var("ADDRESS").unwrap();
            let port = std::env::var("PORT").unwrap();

            let created_hashed_signature:CreateHashedSignatureResult = create_hashed_signature(
                &project_id_fetch.unwrap(), 
                &duration.unwrap_or_else(|| std::env::var("DEFAULT_DURATION_AS_SECONDS").unwrap().to_string().parse::<u64>().unwrap()),
                &permission.clone().unwrap());
            
            
            let doc: InsertRequest = InsertRequest {
                project_name: project_name.unwrap(), //we use the id when we can find the name
                date_created: created_hashed_signature.date_created.clone(),
                expiration_date: created_hashed_signature.expiration_date.clone(),
                options:  CreateSignaturePostRequestOptions {
                    is_consumable: post_request.is_consumable.clone()
                },
                permission: permission.clone().unwrap()
            };
            
            let insert_request_id = &db.collection::<InsertRequest>(DB_Collection::REQUEST.to_string().as_str()).insert_one(doc, None).await.unwrap().inserted_id.as_object_id().unwrap().to_string();
        
            let generated_url:String = format!("{}:{}/{}?permission={},created={}&expiration{}&nonce={}&signature={}",address, port, insert_request_id, permission.unwrap(),created_hashed_signature.date_created,created_hashed_signature.expiration_date,created_hashed_signature.nonce,created_hashed_signature.hashed_signature_base_64);
            return (StatusCode::CREATED, Json(json!(
                {"data":{
                    "url": generated_url
                }
            })));
        }
    }
    return (StatusCode::BAD_REQUEST, Json(json!(
        {"data":None::<String>, "message":"Failed to create signed-url"}
    )));

}

