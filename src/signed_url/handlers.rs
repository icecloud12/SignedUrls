use std::{num::ParseIntError, str::FromStr, time::{SystemTime, UNIX_EPOCH}};

use axum::{
    body::Bytes, extract::{
        Json, Multipart, Path
    }, http::request, response:: IntoResponse
};
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use crate::{network, project::{self, models::ProjectDocument}, request::model::RequestDocument, signed_url::actions::save_files_to_directory};

use super::actions::hash_parameters;
use hyper::StatusCode;
use serde_json::{from_str, json};
use network::db_connection::DATABASE;
use network::DbCollection;
use crate::util::collection::MongoDbCollection;
pub async fn process_signed_url_request(
    Path(params):Path<Vec<(String,String)>>,
    mut multipart: Multipart,
)-> impl IntoResponse{
    //validate the url
    let collected_params = params.iter().map(|param| param.1.clone()).collect::<Vec<String>>();
    let (
        request_id,
        permission,
        created,
        expiration,
        nonce,
        signature
    ) = (
        collected_params[0].to_owned(),
        collected_params[1].to_owned(),
        collected_params[2].to_owned(),
        collected_params[3].to_owned(),
        collected_params[4].to_owned(),
        collected_params[5].to_owned()
    );
    let created_time:Result<u64, ParseIntError> = created.parse();
    let expiration_time:Result<u64, ParseIntError> = expiration.parse();
    if created_time.is_ok() && expiration_time.is_ok() {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if current_time >= created_time.unwrap() && current_time <= expiration_time.unwrap() {
        //fetch project_id from request id
            let db: &Database = DATABASE.get().unwrap();
            let request_entry = db.collection::<RequestDocument>(DbCollection::REQUEST.to_string().as_str()).find_one(doc!{"_id":ObjectId::from_str(&request_id).unwrap()}, None).await;

            if request_entry.is_ok(){
                let project_name = request_entry.unwrap().unwrap().project_name;
                let project_entry = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {"name": project_name}, None).await.unwrap().unwrap();

                let replicated_hash = hash_parameters(&project_entry._id.to_string(), &from_str::<u64>(&created).unwrap(), &from_str::<u64>(&expiration).unwrap(), &permission, &from_str::<u64>(&nonce).unwrap());
                if(replicated_hash == signature){
                    //if valid extract the necessary information
                    let mut file_bytes:Vec<Bytes>= vec![];
                    let mut file_options:Vec<String>=vec![];
                    let mut file_names:Vec<String> = vec![];
                    println!("hashed signature valid");
                    while let Some(mut part) = multipart.next_field().await.unwrap() {
                        
                        if part.name().unwrap_or_else(|| "") == "files" {
                            
                            let file_name = part.file_name().unwrap().to_string();
                            let data: Bytes = part.bytes().await.unwrap();
                            //assume content type containing parts are files
                            file_bytes.push(data);
                            file_names.push(file_name);

                        }else if part.name().unwrap_or_else(|| "") == "fileOptions"{
                            let data = part.bytes().await.unwrap();
                            let val:String = String::from_utf8(data.to_vec()).unwrap();
                            file_options.push(val);
                        }

                    }
                    let _save = save_files_to_directory(file_names, file_bytes, file_options).await;
                    return (StatusCode::OK, Json(json!({"data":{
                        "params":params,
                    }})));
                }else{
                    println!("hashed signature invalid");
                    return (StatusCode::BAD_REQUEST, Json(json!({"data":"Hashed Signature not authorized"})));
                }
            

            }else{
                println!("hashed signature invalid");
                return (StatusCode::BAD_REQUEST, Json(json!({"data":"Hashed Signature not authorized"})));
            }

        }
    }
    println!("request expired");
    return (StatusCode::BAD_REQUEST, Json(json!({"data":"Request Expired"})));
    
}