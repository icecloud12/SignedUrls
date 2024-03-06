use axum::{body::Bytes, extract::{multipart, Multipart}, response::IntoResponse, Error, Json};
use hyper::StatusCode;
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use serde_json::{from_str, json};
use sha3::{Digest, Sha3_256};
use rand::{self, Rng};
use std::{
    num::ParseIntError, path::Path, str::FromStr, time::{
        SystemTime, UNIX_EPOCH
    }
};
use crate::{network::{db_connection::DATABASE, DbCollection}, project::models::ProjectDocument};

use tokio::{fs::File, io::AsyncWriteExt};
use crate::{request::model::RequestDocument};
pub enum ActionTypes {
    UPLOAD,
}

impl ToString for ActionTypes{
    fn to_string(&self)->String{
        match &self {
            &Self::UPLOAD => "upload".to_string()
        }
    }
}

pub struct CreateHashedSignatureResult{
    pub hashed_signature_base_64:String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub nonce: u64
}
pub fn create_hashed_signature(
    project_id:&String,
    duration: &u64,
    action_type: &String,
)-> CreateHashedSignatureResult {
    let date_created:u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // let default_duration_offset: u64 = std::env::var("DEFAULT_DURATION").unwrap().parse::<u64>().unwrap();
    let expiration_date: u64 = date_created + duration;
    let nonce: u64 = rand::thread_rng().gen();
    let hashed_signature_base_64 = hash_parameters(&project_id, &date_created, &expiration_date, &action_type, &nonce);
    
    return CreateHashedSignatureResult {
        hashed_signature_base_64,
        date_created,
        expiration_date,
        nonce
    };
}
pub fn hash_parameters(
    project_id:&String,
    date_created: &u64,
    expiration_date: &u64,
    action_type: &String,
    nonce: &u64,
)-> String{
    let sk = std::env::var("SECRET_KEY").unwrap().to_string();
    let mut hasher = Sha3_256::new();
    println!("{:#?}|{:#?}|{:#?}|{:#?}|{:#?}|{:#?}",project_id,date_created,expiration_date,action_type,nonce, sk);
    hasher.update(&project_id.as_bytes());
    hasher.update((&date_created).to_be_bytes());
    hasher.update((&expiration_date).to_be_bytes());
    hasher.update(&action_type.as_bytes());
    hasher.update(&nonce.to_be_bytes());
    let signature = hasher.finalize();
    let hashed_signature_base_64 = signature.as_slice().iter().map(|b| format!("{:02x}", b)).collect::<String>();
    return hashed_signature_base_64;
}

pub struct ValidateSignedUrlResultUploadFiles {
    bytes: Vec<Bytes>,
    names: Vec<String>,
    options: Vec<String>

}
pub async fn validate_signed_url( 
    params: Vec<(String,String)>,
    mut multipart: Multipart
) -> Result<ValidateSignedUrlResultUploadFiles, String> {
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
                    return Ok(ValidateSignedUrlResultUploadFiles {
                        bytes: file_bytes,
                        names: file_names,
                        options: file_options
                    });
                }else{
                    println!("hashed signature invalid");
                    return Err("hashed signature invalid".to_string());
                }
            

            }else{
                println!("hashed signature invalid");
                return Err("Hashed Signature not authorized".to_string());
            }

        }
    }
    println!("hashed signature invalid");
    return Err("Hashed Signature not authorized".to_string());
}

pub async fn save_files_to_directory(

    result: ValidateSignedUrlResultUploadFiles

)-> Result<(), bool>{
    //the request itself should have a file path to go to
    for (index, element) in result.bytes.iter().enumerate(){
        let file_name = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();
        
        println!("fileNames: {}", result.names[index].as_str());
        let splits = result.names[index].as_str().split(".");
        let file_extention = splits.last().unwrap();
        println!("{}", file_extention);
        let mut file:File = File::create(format!("./data/{}.{}",file_name,file_extention)).await.unwrap();
        file.write(&element).await.unwrap();
    }
    Ok(())
}

