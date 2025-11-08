use crate::{
    models::{
        self, file_models::{FileDocumentInsertRow, FileDocumentOptions}, project_models::ProjectDocument, request_models::{MergeRequestOptions, RequestDocument, RequestDocumentOptions, RequestOptions, RequestQueryParamsV2, UploadRequestDocument, ViewFileRequestDocument, ViewFileRequestOptions}, signed_url_models::{self, File, SaveFilesToDirectoryResult}
    },
    network::{db_connection::DATABASE, DbCollection},
    request::actions::file_reference_is_valid,
};
use axum::{body::Bytes, extract::Multipart};
use hyper::StatusCode;
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::InsertOneResult,
    Database,
};
use rand::{self, Rng};
use serde_json::from_str;
use sha2::{Digest, Sha256};
use signed_urls::collections;
use std::{
    num::ParseIntError, path::PathBuf, str::FromStr, time::{SystemTime, UNIX_EPOCH}
};

use tokio::{
    fs::{self, File as TokioFile},
    io::AsyncWriteExt,
};
pub enum ActionTypes {
    UPLOAD_V1,
    UPLOAD_V2,
    VIEW_V1,
    VIEW_V2,
    DELETE_V1,
    DELETE_V2,
}

pub enum UploadActionTypes{
    V1,
    V2
}
#[derive(Copy, Clone)]
pub enum ViewActionTypes {
    V1,
    V2
}

impl From<ViewActionTypes> for ActionTypes{
    fn from(value: ViewActionTypes) -> Self {
         match value {
             ViewActionTypes::V1 => ActionTypes::VIEW_V1,
             ViewActionTypes::V2 => ActionTypes::VIEW_V2
         }   
    }
}
impl ToString for ViewActionTypes{
    fn to_string(&self) -> String {
        match &self {
            Self::V1 => ActionTypes::VIEW_V1.to_string(),
            Self::V2 => ActionTypes::VIEW_V2.to_string()
        }
    }
}

impl ToString for ActionTypes {
    fn to_string(&self) -> String {
        match &self {
            &Self::UPLOAD_V1 => "upload_v1".to_string(),
            &Self::UPLOAD_V2 => "upload_v2".to_string(),
            &Self::VIEW_V1 => "view_v1".to_string(),
            &Self::VIEW_V2 => "view_v2".to_string(),
            &Self::DELETE_V1 => "delete_v1".to_string(),
            &Self::DELETE_V2 => "delete_v2".to_string(),
        }
    }
}
impl TryFrom<ActionTypes> for UploadActionTypes{
    type Error = String;
    fn try_from(value: ActionTypes) -> Result<Self, Self::Error> {
        match value {
            ActionTypes::UPLOAD_V1 => Ok(UploadActionTypes::V1),
            ActionTypes::UPLOAD_V2 => Ok(UploadActionTypes::V2),
            _ => Err(format!("Not a valid enum type and version"))
        }
    }
}
impl TryInto<ViewActionTypes> for ActionTypes{
    type Error = StatusCode;
    fn try_into(self) -> Result<ViewActionTypes, Self::Error> {
        match self {
            ActionTypes::VIEW_V1 => Ok(ViewActionTypes::V1),
            ActionTypes::VIEW_V2 => Ok(ViewActionTypes::V2),
            _ => Err(StatusCode::BAD_REQUEST)
        }
    }
}

pub struct CreateHashedSignatureResult {
    pub hashed_signature_base_64: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub nonce: u64,
}
pub fn create_hashed_signature(
    project_id: &String,
    duration: &u64,
    action_type: &String,
    file_id: Option<&String>,
) -> CreateHashedSignatureResult {
    let date_created: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // let default_duration_offset: u64 = std::env::var("DEFAULT_DURATION").unwrap().parse::<u64>().unwrap();
    let expiration_date: u64 = date_created + duration;
    let nonce: u64 = rand::thread_rng().gen();
    let hashed_signature_base_64 = hash_parameters(
        &project_id,
        &date_created,
        &expiration_date,
        &action_type,
        &nonce,
        file_id,
    );

    return CreateHashedSignatureResult {
        hashed_signature_base_64,
        date_created,
        expiration_date,
        nonce,
    };
}
pub fn hash_parameters(
    project_id: &String,
    date_created: &u64,
    expiration_date: &u64,
    action_type: &String,
    nonce: &u64,
    file_id: Option<&String>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&project_id.as_bytes());
    hasher.update((&date_created).to_be_bytes());
    hasher.update((&expiration_date).to_be_bytes());
    hasher.update(&action_type.as_bytes());
    hasher.update(&nonce.to_be_bytes());
    if file_id.is_some() {
        hasher.update(file_id.unwrap().as_bytes());
    }
    let signature = hasher.finalize();
    let hashed_signature_base_64 = signature
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    return hashed_signature_base_64;
}

pub async fn validate_signed_url_v1(params: Vec<String>, permission: &str) -> bool {
    let (request_id, created, expiration, nonce, signature) = (
        params[0].to_owned(),
        params[1].to_owned(),
        params[2].to_owned(),
        params[3].to_owned(),
        params[4].to_owned(),
    );

    let created_time: Result<u64, ParseIntError> = created.parse();
    let expiration_time: Result<u64, ParseIntError> = expiration.parse();
    if created_time.is_ok() && expiration_time.is_ok() {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if current_time >= created_time.unwrap() && current_time <= expiration_time.unwrap() {
            //fetch project_id from request id
            let db: &Database = DATABASE.get().unwrap();
            let request_entry = db
                .collection::<RequestDocument>(DbCollection::REQUEST.to_string().as_str())
                .find_one(doc! {collections::Request::ID: ObjectId::from_str(&request_id).unwrap()}, None)
                .await;
            if request_entry.is_ok() {
                let entry = request_entry.unwrap().unwrap();

                let project_id = entry.project_id;
                let project_entry = db
                    .collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str())
                    .find_one(doc! { collections::Bucket::ID : project_id}, None)
                    .await
                    .unwrap()
                    .unwrap();

                let replicated_hash = hash_parameters(
                    &project_entry._id.to_string(),
                    &from_str::<u64>(&created).unwrap(),
                    &from_str::<u64>(&expiration).unwrap(),
                    &permission.to_string(),
                    &from_str::<u64>(&nonce).unwrap(),
                    None,
                );
                if replicated_hash == signature {
                    if entry.permission == ActionTypes::UPLOAD_V1.to_string() {
                        let options: RequestDocumentOptions =
                            entry.options.unwrap();
                        if options.is_consumed {
                            return false;
                        }else{ 
                            if options.is_consumable {
                                let filter = doc! {collections::Request::ID : entry._id};
                                let update = doc! {
                                    "$set": { "options.is_consumed" : true }
                                };

                                let _update_result = db
                                    .collection::<RequestDocument>(
                                        DbCollection::REQUEST.to_string().as_str(),
                                    )
                                    .update_one(filter, update, None)
                                    .await
                                    .unwrap();
                            }
                            return true;
                        }
                    } else if entry.permission == ActionTypes::VIEW_V1.to_string() {
                        //we we're unable to consume the request because the request(and hash) is shared between requested files
                        return true;
                    }
                    return false;
                } else {
                    //replicated_hash != signature
                    return false;
                }
            }
        }
    }
    return false;
}
pub async fn validate_signed_url_v2(
    file_id: Option<&String>,
    query: RequestQueryParamsV2,
    permission: ActionTypes,
) -> bool {

    //check param files are all Some
    let RequestQueryParamsV2 {
        request,
        created,
        expiration,
        nonce,
        signature,
    } = query;
    match permission {
        ActionTypes::VIEW_V2 => {
            if file_id.is_none() {
                return false;
            } else {
                let file_id = file_id.unwrap();

                match ObjectId::from_str(&file_id) {
                    Ok(file_object_id) => {
                        //valid objectid syntax
                        match file_reference_is_valid(file_object_id).await {
                            Ok(file) => {
                                //skip checking hash if file is public
                                if file.options.is_public {
                                    return true;
                                } else {
                                    match (request, created, expiration, nonce, signature) {
                                        (
                                            Some(request_id),
                                            Some(created),
                                            Some(expiration),
                                            Some(nonce),
                                            Some(signature),
                                        ) => {
                                            let current_time: u64 = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs();
                                            if current_time >= created && current_time <= expiration {
                                                let project_id = file.project_id.to_hex();
                                                if  replicate_hash(&project_id, Some(&file_id), &created, &expiration, &nonce, &permission, signature){
                                                    let db = DATABASE.get().unwrap();
                                                    //get request record
                                                    let request_object_id =
                                                        ObjectId::from_str(&request_id).unwrap();
                                                    let request_record = db
                                                        .collection::<RequestDocument>(
                                                            DbCollection::REQUEST
                                                                .to_string()
                                                                .as_str(),
                                                        )
                                                        .find_one(
                                                            doc! {
                                                                collections::Request::ID : request_object_id
                                                            },
                                                            None,
                                                        )
                                                        .await;
                                                    match request_record {
                                                        Ok(option_record) => {
                                                            match option_record {
                                                                Some(record) => {
                                                                    if record.permission == permission.to_string() {
                                                                        match db.collection::<ViewFileRequestDocument>(
                                                                            DbCollection::VIEW_FILE_REQUEST
                                                                                .to_string()
                                                                                .as_str(),
                                                                            )
                                                                            .find_one( doc! { collections::ViewFileRequest::FILE_ID: &file_object_id, collections::ViewFileRequest::REQUEST_ID: &request_object_id}, None).await {
                                                                                Ok(view_file_request_document_option) => {
                                                                                    match view_file_request_document_option {
                                                                                        Some(view_file_request) => {
                                                                                            let request_options =
                                                                                                view_file_request.options;
                                                                                            match request_options {
                                                                                                None => {
                                                                                                    return false;
                                                                                                }
                                                                                                Some(options) => {
                                                                                                    let ViewFileRequestOptions{is_consumable, is_consumed} = options;
                                                                                                    if is_consumable {
                                                                                                        if is_consumed {
                                                                                                            return false;
                                                                                                        } else {
                                                                                                            //consume the request
                                                                                                            let filter = doc! {collections::ViewFileRequest::ID: view_file_request._id};
                                                                                                            let update = doc! {"$set": { "options.is_consumed":  true }};
                                                                                                            let _ = db.collection::<ViewFileRequestDocument>(DbCollection::VIEW_FILE_REQUEST.to_string().as_str())
                                                                                                                .update_one(filter,update,None).await;
                                                                                                            return true;
                                                                                                        }
                                                                                                    } else {
                                                                                                        return true;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        None => return false,
                                                                                    }
                                                                                }
                                                                                Err(_) => {
                                                                                    return false;
                                                                                }
                                                                            }
                                                                    } else {
                                                                        return false;
                                                                    }
                                                                }
                                                                None => return false,
                                                            }
                                                        }
                                                        Err(_) => {
                                                            return false;
                                                        }
                                                    }
                                                } else {
                                                    return false;
                                                }
                                            } else {
                                                return false;
                                            }
                                            //get action type of request
                                        }
                                        _ => return false,
                                    }
                                }
                            }
                            Err(_) => return false,
                        }
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
        }
        ActionTypes::UPLOAD_V2 => {
            match (request, created, expiration, nonce, signature) {
                (Some(request), Some(created), Some(expiration), Some(nonce), Some(signature))=>{
                    let current_time: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    if current_time >= created && current_time <= expiration {
                        let db: &Database = DATABASE.get().unwrap();
                        let object_id = ObjectId::from_str(request.as_str()).unwrap();
                        let find_result = db.collection::<UploadRequestDocument>(DbCollection::REQUEST.to_string().as_str()).find_one(
                            doc!{collections::Request::ID: object_id}, None
                        ).await;
                        match find_result {
                            Ok(request_doc_o)=>{
                                match request_doc_o {
                                    Some(request_doc)=>{
                                        if replicate_hash(&request_doc.project_id.to_hex(), None, &created, &expiration, &nonce, &permission, signature){
                                            let RequestOptions{ is_consumable, is_consumed, ..} = request_doc.options; 
                                            match (is_consumable, is_consumed) {
                                                (Some(is_consumable), is_consumed)=>{
                                                    let is_consumed = is_consumed.unwrap_or_else(|| false);
                                                    if is_consumable {
                                                        if is_consumed { return true }
                                                        else {
                                                            //consume it
                                                            let filter = doc!{ signed_urls::collections::Bucket::ID.to_string().as_str(): request_doc._id};
                                                            let update = doc!{ "$set": { "options.is_consumed": true }};
                                                            let _ =db.collection::<UploadRequestDocument>(DbCollection::REQUEST.to_string().as_str()).update_one(filter, update, None);
                                                            return true;
                                                        }
                                                    } else {return true}
                                                }
                                                (None, _) => { return true;}
                                            }
                                        } else{
                                            tracing::error!("Failed to replicate hash");
                                            return false;
                                        }
                                    }
                                    None => return false
                                }
                            }
                            Err(mongodb_error) => {
                                
                                tracing::error!("{:#?}",mongodb_error);
                                return false;
                            }//this should return status code
                        }
                        //query is ok and returns 1 document
                    } else {
                        tracing::info!("request expired, exceeded valid range");
                        return false;
                    }
                    
                }
                _ => { return false; }
            }
        }
        _ => return false,
    }

    //check file is valid
    //check file_id is private
}

pub async fn save_files_to_directory(
    request_id: &String,
    project_id: &String,
    target: Option<String>,
    options: Option<RequestOptions>,
    mut multipart: Multipart,
    version: UploadActionTypes
    //param: ValidateSignedUrlResultUploadFiles
) -> Result<SaveFilesToDirectoryResult, bool> {
    let mut initial_path: std::path::PathBuf = std::path::PathBuf::from("./data/")
        .join(format!("{}/", project_id.clone()));
    if let Some(target) = target{
        
        initial_path = initial_path.join(format!("{}/", target));
    }

    let mut created_files: Vec<signed_url_models::File> = vec![];
    let key = if let UploadActionTypes::V1 = version { "files" } else { "file" };
    let merge_options = match options {
        None => {MergeRequestOptions::default()}
        Some(r_options) => {
            MergeRequestOptions::from(r_options)
        }
    };
    while let Some(mut part) = multipart.next_field().await.unwrap() {
        if (part.name().unwrap_or_else(|| "")) == key {
            match part.file_name() {
                Some(file_name) => {
                    let mut chunks: Vec<Bytes> = Vec::new();
                    //file_name.to_string()
                    let original_file_name = file_name.to_string();
                    // let file_bytes = part.bytes().await.unwrap();
                    while let Some(streamed_chunk) = &part.chunk().await.unwrap() {
                        chunks.push(streamed_chunk.to_owned());
                    }
                     
                    let file_document_insert: FileDocumentInsertRow = FileDocumentInsertRow {
                        file_name: original_file_name.clone(),
                        path: initial_path.to_str().unwrap().to_string(),
                        options: FileDocumentOptions {
                            is_public: merge_options.is_public,
                        },
                        project_id: ObjectId::from_str(project_id.as_str()).unwrap(),
                        request_id: ObjectId::from_str(request_id.as_str()).unwrap(),
                    };
                    let db: &Database =  DATABASE.get().unwrap();
                    let insert_file_insert_result: InsertOneResult = db
                        .collection::<FileDocumentInsertRow>(
                            DbCollection::FILE.to_string().as_str(),
                        )
                        .insert_one(file_document_insert, None)
                        .await
                        .unwrap();
                    let new_file_name = insert_file_insert_result
                        .inserted_id
                        .as_object_id()
                        .unwrap()
                        .to_string();
                    let new_file_directory: PathBuf =
                        initial_path.join(format!("{}/", new_file_name));
                    if !( fs::metadata(&new_file_directory).await.is_ok()
                        && fs::metadata(&new_file_directory).await.expect("").is_dir()) {

                        match std::fs::create_dir_all(&new_file_directory) {
                            Ok(_a) => {
                                //do something on dir creation
                                let file_extention = original_file_name.split(".").last().unwrap();
                                let new_file_path: String = new_file_directory
                                    .clone()
                                    .join(format!("{}.{}", new_file_name, file_extention))
                                    .to_str()
                                    .unwrap()
                                    .to_string();

                                let saved_file = save_file_to_directory(
                                    original_file_name,
                                    new_file_name,
                                    new_file_path,
                                    chunks,
                                )
                                .await;
                                created_files.push(saved_file);
                            }
                            Err(_) => {
                                println!("cannot create in this directory");
                                return Err(false);
                            }
                        }
                    }
                }
                None => {}
            };
        }
    }

    Ok(SaveFilesToDirectoryResult {
        request_id: request_id.to_string(),
        path: initial_path
            .to_str()
            .unwrap()
            .to_string()
            .split("/data")
            .last()
            .unwrap()
            .to_string(),
        files: created_files,
    })
}

pub async fn save_file_to_directory(
    original_file_name: String,
    new_file_name: String,
    new_file_path: String,
    file_chunks: Vec<Bytes>,
) -> signed_url_models::File {
    let mut file: TokioFile = TokioFile::create(new_file_path.clone()).await.unwrap();
    let mut _i = file_chunks.iter();
    while let Some(file_chunk) = _i.next() {
        let _ = file.write_all(&file_chunk).await;
    }

    return models::signed_url_models::File {
        _id: new_file_name,
        file_name: original_file_name,
        path: new_file_path.split("./data").last().unwrap().to_string(),
    };
}

pub fn replicate_hash(project_id: &String, file_id: Option<&String>, created: &u64, expiration: &u64, nonce: &u64, permission: &ActionTypes, signature: String)->bool {
    let replicated_hash = hash_parameters(
        project_id,
        created,
        expiration,
        &permission.to_string(),
        nonce,
        file_id,
    );
    replicated_hash == signature
}
