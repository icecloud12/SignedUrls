use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    response:: IntoResponse,
    extract::Json
};
use mongodb::Database;
use serde::Deserialize;
use crate::network::DB_Collection;
use crate::network::Db_Connection::DATABASE;
use crate::actions::project;
use crate::models::Request::CreateSignedUrlPostRequest;
use mongodb::bson::doc;
// #[derive(Deserialize)]
// pub struct CreateSignedUrlPostRequest {
//     pub project_name: String,
//     pub duration: Option<u64>, // defaults 25200
//     pub is_consumable: Option<bool> //defaults false
// }


pub async fn create_request(Json(post_request):Json<CreateSignedUrlPostRequest>) -> impl IntoResponse{
    let db: &Database = DATABASE.get().unwrap();
    let project_id:Option<String> = project::get_project_id_by_name(&post_request.project_name).await;
    let date_created:u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    //other web services uses 7 days but i think its a bit too much
    let default_duration_offset: u64 = 25200;
    let expiration_date: u64 = match post_request.duration {
        Some(duration) => {
            let d = date_created + duration;
            d
        },
        None => {
            let d = date_created + default_duration_offset;
            d
        }
    };
    // let doc = !doc {

    // };
    // db.collection(DB_Collection::REQUEST.to_string().as_str()).insert_one(doc, None)

}

