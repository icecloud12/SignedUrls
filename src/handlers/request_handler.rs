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
use crate::actions::project;
use crate::models::Request::{CreateSignedUrlPostRequest, InsertRequest, CreateSignaturePostRequestOptions};
use mongodb::bson::doc;



pub async fn create_request(Json(post_request):Json<CreateSignedUrlPostRequest>) -> impl IntoResponse{
    let db: &Database = DATABASE.get().unwrap();
    if post_request.project_name.is_some() {
        let project_name = post_request.project_name.unwrap(); 
        
        let project_id_fetch = project::get_project_id_by_name(&project_name).await;
        if project_id_fetch.is_some() {

            let project_id = project_id_fetch.unwrap();
            let date_created:u64 = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            //other web services uses 7 days but i think its a bit too much
            let default_duration_offset: u64 = 25200;
            let expiration_date: u64 = date_created + post_request.duration.unwrap_or_else(|| default_duration_offset);
            let nonce: u64 = rand::thread_rng().gen();
            
            let mut hasher = Sha3_256::new();
            //I dont understand the use of a secret key where we can also have nonce making this an ephemeral
            hasher.update(project_id.as_bytes());
            hasher.update((&date_created).to_be_bytes());
            hasher.update((&expiration_date).to_be_bytes());
            hasher.update(&nonce.to_be_bytes());
            
            let signature = hasher.finalize();
            let doc = InsertRequest {
                project_name: project_name,
                date_created: date_created,
                epiration_date: expiration_date,
                options:  CreateSignaturePostRequestOptions {
                    is_consumable: post_request.is_consumable
                }
            };
            let insert_request_id = &db.collection::<InsertRequest>(DB_Collection::REQUEST.to_string().as_str()).insert_one(doc, None).await.unwrap().inserted_id.as_object_id().unwrap().to_string();
        
            let address=std::env::var("ADDRESS").unwrap();
            let port = std::env::var("PORT").unwrap();
        
            let signature_base64 = signature.as_slice().iter().map(|b| format!("{:02x}", b)).collect::<String>();
        
            let generated_url:String = format!("{}:{}/{}?created={}&expiration{}&nonce={}&signature={}",address, port, insert_request_id,date_created, expiration_date, nonce, signature_base64);
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

