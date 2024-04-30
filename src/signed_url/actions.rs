use axum::{body::Bytes, extract::{multipart,  Multipart}, Error};
use mongodb::{bson::{doc, oid::ObjectId}, results::InsertOneResult, Database};
use serde_json::from_str;
use sha3::{Digest, Sha3_256};
use rand::{self, Rng};
use std::{
    f32::consts::E, fs, num::ParseIntError, os::windows::fs::MetadataExt, path::PathBuf, str::FromStr, time::{
        SystemTime, UNIX_EPOCH
    }
};
use super::models::SaveFilesToDirectoryResult;
use crate::{ file::{self, model::FileDocumentInsertRow}, network::{db_connection::DATABASE, DbCollection}, project:: models::ProjectDocument, request::model::UploadRequestDocument};

use tokio::{fs::File, io::AsyncWriteExt};
use crate::request::model::RequestDocument;
use crate::file::model::FileDocumentOptions;
pub enum ActionTypes {
    UPLOAD,
    VIEW,
    DIRECT_UPLOAD,
    DIRECT_VIEW
}

impl ToString for ActionTypes{
    fn to_string(&self)->String{
        match &self {
            &Self::UPLOAD => "upload".to_string(),
            &Self::VIEW => "view".to_string(),
            &Self::DIRECT_UPLOAD => "direct_upload".to_string(),
            &Self::DIRECT_VIEW => "direct_view".to_string()
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

pub async fn validate_signed_url( 
    params: Vec<String>,
    permission: &str
) -> bool {
    let (
        request_id,
        created,
        expiration,
        nonce,
        signature
    ) = (
        params[0].to_owned(),
        params[1].to_owned(),
        params[2].to_owned(),
        params[3].to_owned(),
        params[4].to_owned()
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
                let entry = request_entry.unwrap().unwrap();
                
                let project_id = entry.project_id;
                let project_entry = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {"_id": ObjectId::from_str(&project_id).unwrap()}, None).await.unwrap().unwrap();

                let replicated_hash = hash_parameters(&project_entry._id.to_string(), &from_str::<u64>(&created).unwrap(), &from_str::<u64>(&expiration).unwrap(), &permission.to_string(), &from_str::<u64>(&nonce).unwrap());
                if replicated_hash == signature{
                    if entry.permission == ActionTypes::UPLOAD.to_string(){
                        let options: crate::request::model::RequestDocumentOptions  = entry.options.unwrap();
                        if options.is_consumable {
                            let filter = doc! {"_id": entry._id};
                            let update = doc!{
                                "$set": {
                                    "options" : {
                                        //you actually need to restructure it damn, TAKE NOTE OF UPDATING a SUBOBJECT
                                        "is_consumbable": options.is_consumable,
                                        "is_consumed": true,
                                        "is_public": options.is_public
                                    }
                                }
                            };
                            
                            let _update_result = db.collection::<ProjectDocument>(DbCollection::REQUEST.to_string().as_str()).update_one(
                                filter,
                                update,
                                None
                            ).await.unwrap();
                        }
                        return true;
                    }else if entry.permission == ActionTypes::VIEW.to_string(){
                        return true;
                    }
                    return false;
                 
                } else { //replicated_hash != signature
                    return false;
                }
            }
        }
    }
    return false;
}


pub async fn save_files_to_directory(
    request_id:String,
    mut multipart:Multipart,
    //param: ValidateSignedUrlResultUploadFiles
)-> Result<SaveFilesToDirectoryResult, bool>{
    //the request itself should have a file path to go to
    //get the project name and target path by files.request_id

    let db: &Database = DATABASE.get().unwrap();
    let request_entry = db.collection::<UploadRequestDocument>(DbCollection::REQUEST.to_string().as_str()).find_one(doc!{"_id": ObjectId::from_str(request_id.as_str()).unwrap()}, None).await.unwrap().unwrap();

    let project_id = request_entry.project_id;
    let project_doc = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc!{"_id": ObjectId::from_str(project_id.as_str()).unwrap()}, None).await.unwrap().unwrap();
    let project_id = project_doc._id.to_hex();
    let initial_path: std::path::PathBuf = std::path::PathBuf::from("./data/").join(format!("{}/",project_id.clone())).join(format!("{}/",request_entry.target));

    
    let mut created_files: Vec<super::models::File> = vec![];
    while let Some(part) = multipart.next_field().await.unwrap(){
        if(part.name().unwrap_or_else(|| "")) == "files" {
            
            match part.file_name(){
                Some(file_name) => {
                    //file_name.to_string()
                    let original_file_name = file_name.to_string();
                    let file_bytes = part.bytes().await.unwrap();
                    let is_public = request_entry.options.is_public.unwrap_or_else(|| false);
                    let file_document_insert: FileDocumentInsertRow = FileDocumentInsertRow {
                        file_name : original_file_name.clone(),
                        path: initial_path.to_str().unwrap().to_string(),
                        options: FileDocumentOptions{
                            is_public: is_public
                        },
                        project_id: ObjectId::from_str(project_id.as_str()).unwrap(),
                        request_id: request_entry._id,
                    };
                    let insert_file_insert_result: InsertOneResult = db.collection::<FileDocumentInsertRow>(DbCollection::FILE.to_string().as_str()).insert_one(file_document_insert, None).await.unwrap();
                    let new_file_name = insert_file_insert_result.inserted_id.as_object_id().unwrap().to_string();
                    let new_file_directory:PathBuf = initial_path.join(format!("{}/", new_file_name));
                    if !(fs::metadata(&new_file_directory).is_ok() && fs::metadata(&new_file_directory).expect("").is_dir()){
                        match std::fs::create_dir_all(&new_file_directory) {
                            Ok(_a) =>{
                                //do something on dir creation
                                let file_extention = original_file_name.split(".").last().unwrap();
                                let new_file_path: String = new_file_directory.clone().join(format!("{}.{}",new_file_name,file_extention)).to_str().unwrap().to_string();
                                
                                let saved_file = save_file_to_directory(original_file_name, new_file_name, new_file_path, file_bytes).await;
                                created_files.push( saved_file);
                            },
                            Err(_) => {
                                println!("cannot create in this directory");
                                return Err(false)
                            }
                        }
                    }
                    
                },
                None => {}
            };
            
        }
        
    }


    Ok(SaveFilesToDirectoryResult {
        request_id: request_id,
        path: initial_path.to_str().unwrap().to_string().split("/data").last().unwrap().to_string(),
        files: created_files
    })
}

pub async fn save_file_to_directory(original_file_name:String, new_file_name:String, new_file_path:String, file_bytes:Bytes)-> super::models::File{
    let mut file:File = File::create(new_file_path.clone()).await.unwrap();
    file.write(&file_bytes).await.unwrap();
    return super::models::File{
        _id: new_file_name,
        file_name:original_file_name,
        path: new_file_path.split("./data").last().unwrap().to_string()
    }
}
pub struct multipartFile {
    pub file_name: String,
    pub bytes: Bytes,
    pub file_size_is_valid:bool
}
pub struct InterceptedFile {
    pub file_name: String,
    pub bytes: Bytes,
}
pub async fn direct_upload_extract_multipart(mut multipart:Multipart)-> Result<(Vec<multipartFile>, String, bool, bool, usize), String> {
    let mut multipart_files:Vec<multipartFile> = Vec::new();
    let mut intercepted_files:Vec<InterceptedFile> = Vec::new();

    let mut target: String = String::new();
    let mut is_public:bool = false; //default
    let mut is_consumable:bool = false;//default
    let mut max_file_size:usize = usize::MAX;
    
    let mut atleast_one_file: bool = false;
    let mut target_is_set:bool = false;
    let mut max_size_is_ok:bool = true;


    while let Some(mut part)  =  multipart.next_field().await.unwrap(){
        
        match part.name() {
            Some(part_name)=>{
                if part_name == "files" {
                    let file_name = &part.file_name().unwrap().to_string();
                    let file_bytes = &part.bytes().await.unwrap();
                    
                    
                    let file_size_is_valid = file_bytes.to_vec().len() <= max_file_size;
                    intercepted_files.push(InterceptedFile {
                        file_name:  file_name.clone(),
                        bytes: file_bytes.clone(),
                        //file_size_is_valid: file_size_is_valid
                    });
                    atleast_one_file = true;
                 
                } else if part_name == "target"{
                    target = std::str::from_utf8(part.bytes().await.unwrap().into_iter().collect::<Vec<u8>>().as_ref()).unwrap().to_string();
                    target_is_set = true;
                } else if part_name == "is_public" {
                    is_public = std::str::from_utf8(part.bytes().await.unwrap().into_iter().collect::<Vec<u8>>().as_ref()).unwrap() == "true".to_string();

                } else if part_name == "is_consumable" {
                    is_consumable = std::str::from_utf8(part.bytes().await.unwrap().into_iter().collect::<Vec<u8>>().as_ref()).unwrap() == "true".to_string()
                } else if part_name == "max_file_size" {//in bytes
                    match std::str::from_utf8(part.bytes().await.unwrap().into_iter().collect::<Vec<u8>>().as_ref()).unwrap().to_string().parse::<usize>(){
                        Ok(val) => {
                            max_file_size = val;
                        },
                        Err(_) => {
                            max_size_is_ok = false
                        }
                        
                    }
                }
            },
            None =>{
            }

        }
    }
    for intercepted_file in intercepted_files {
            let file_size_is_valid = intercepted_file.bytes.to_vec().len() <= max_file_size;
            multipart_files.push(multipartFile {
                file_name:  intercepted_file.file_name.clone(),
                bytes: intercepted_file.bytes.clone(),
                file_size_is_valid: file_size_is_valid
            });
    }

    if atleast_one_file && target_is_set && max_size_is_ok{
        return Ok((multipart_files, target, is_public,is_consumable, max_file_size ))
    }else{
        if !atleast_one_file {
            return Err("no files sent".to_string())
        }else if !target_is_set {
            return Err("target is not set".to_string())
        }else{
            return Err("max_file_size value is not valid".to_string())
        }
    }
}