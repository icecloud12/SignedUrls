use hyper::StatusCode;
use mongodb::{bson::oid::ObjectId};

use serde::{Deserialize, Serialize};

use crate::models::project_models::BucketDocument;

#[derive(Deserialize, Serialize)]
pub struct RequestOptions {
    pub is_consumable: Option<bool>,
    pub is_consumed: Option<bool>,
    pub is_public: Option<bool>,
    pub size_limit: Option<usize>,
    pub mime_types: Option<Vec<String>>
}


pub struct DefaultRequestOptions {
    pub is_consumable: bool,
    pub is_consumed: bool,
    pub is_public: bool,
    pub size_limit: Option<usize>,
    pub mime_types: Vec<String>
    
}

pub struct MergeRequestOptions {
    pub is_consumable: bool,
    pub is_consumed: bool,
    pub is_public: bool,
    pub size_limit: Option<usize>,
    pub mime_types: Vec<String>
}


impl Default for DefaultRequestOptions {
    fn default() -> Self {
        DefaultRequestOptions { is_consumable: true, is_consumed: false, is_public: false, size_limit: None, mime_types: vec![] }
    }
}

impl From<RequestOptions> for MergeRequestOptions{
    fn from(value: RequestOptions) -> Self {
        let DefaultRequestOptions{is_consumable, is_consumed, is_public, mime_types, ..} = DefaultRequestOptions::default();
        MergeRequestOptions{
            is_consumable: value.is_consumable.unwrap_or_else(|| is_consumable),
            is_consumed: value.is_consumed.unwrap_or_else(|| is_consumed),
            is_public: value.is_public.unwrap_or_else(|| is_public),
            size_limit: value.size_limit,
            mime_types: value.mime_types.unwrap_or_else(|| mime_types)
        }
    }
}
impl Default for MergeRequestOptions {
    fn default() -> Self {
        let DefaultRequestOptions { is_consumable, is_consumed, is_public, size_limit, mime_types } = DefaultRequestOptions::default();
        MergeRequestOptions { is_consumable, is_consumed, is_public, size_limit, mime_types }
    }
}


#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequestV1 {
    //pub project_name: Option<String>, //deserialization would throw an error and panics the program (Deprecated and is moved to APIKEY headers)
    pub duration: Option<u64>,  // defaults to env DEFAULT_DURATION variable
    pub target: Option<String>, //target destination appended to the project-name as the path dir to upload,
    pub is_consumable: Option<bool>, //defaults false
    pub is_public: Option<bool>, //when uploaded file becomes a public file where anybody can see
    pub api_key: Option<String>,
}
#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequestV2 { // same as v1 with extra fields
    pub duration: Option<u64>,  // defaults to env DEFAULT_DURATION variable
    pub target: Option<String>, //target destination appended to the project-name as the path dir to upload,
    pub is_consumable: Option<bool>, //defaults false
    pub is_public: Option<bool>, //when uploaded file becomes a public file where anybody can see
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub size_limit: Option<usize>,
    pub mime_types: Option<Vec<String>>
}


#[derive(Deserialize, Serialize)]
pub struct GenericRequest {
    pub project_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options: Option<RequestOptions>,
    pub permission: String,
}

#[derive(Deserialize, Serialize)]
pub struct UploadRequestDocument {
    pub _id: ObjectId,
    pub project_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options: RequestOptions,
    pub permission: String,
    pub target: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct UploadRequest {
    pub project_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options: RequestOptions,
    pub permission: String,
    pub target: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RequestDocument {
    pub _id: ObjectId,
    pub project_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options: Option<RequestDocumentOptions>,
    pub permission: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestDocumentOptions {
    pub is_consumable: bool,
    pub is_consumed: bool,
    pub is_public: bool,
}

//accepts a duration in seconds and a vector of file_ids
#[derive(Deserialize)]
pub struct CreateSignedUrlViewRequest {
    pub duration: Option<u64>,
    pub files: Option<Vec<String>>,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub is_consumable: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateSignedUrlViewRequestV1 {
    pub duration: Option<u64>,
    pub file_id_collection: Option<Vec<String>>,
    pub api_key: Option<String>,
    pub is_consumable: Option<bool>,
}
#[derive(Deserialize)]
pub struct CreateSignedUrlViewRequestV2{
    pub duration: Option<u64>,
    pub files: Option<Vec<String>>,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub is_consumable: Option<bool>,
    
}
impl TryFrom<CreateSignedUrlViewRequestV2> for CreateSignedUrlViewRequest{
    type Error = StatusCode;
    fn try_from(value:CreateSignedUrlViewRequestV2) -> Result<CreateSignedUrlViewRequest, Self::Error> {
        match (&value.public_key, &value.secret_key){
            (Some(_), Some(_))=>{
                let CreateSignedUrlViewRequestV2 { duration, files, public_key, secret_key, is_consumable } = value;
                Ok(CreateSignedUrlViewRequest { duration, files, public_key, secret_key, is_consumable })
            }
            _ => {Err(StatusCode::UNAUTHORIZED)}

        }
    }
}
impl TryFrom<CreateSignedUrlViewRequestV1> for CreateSignedUrlViewRequest{
    type Error = StatusCode;
    fn try_from(value: CreateSignedUrlViewRequestV1) -> Result<Self, Self::Error> {
        if value.api_key.is_some(){
            let CreateSignedUrlViewRequestV1 { duration, file_id_collection, api_key, is_consumable } = value;
            Ok(CreateSignedUrlViewRequest { duration, files: file_id_collection, public_key: api_key, secret_key: None, is_consumable })
        }else{Err(StatusCode::UNAUTHORIZED)}
        
    }
}

#[derive(Deserialize, Serialize)]
pub struct ViewRequest {
    pub project_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub permission: String,
    pub files: Vec<String>,
    pub options: Option<RequestOptions>,
}
#[derive(Deserialize)]
pub struct RequestQueryParamsV2 {
    #[serde(rename="r")]
    pub request: Option<String>,
    #[serde(rename="c")]
    pub created: Option<u64>,
    #[serde(rename="e")]
    pub expiration: Option<u64>,
    #[serde(rename="n")]
    pub nonce: Option<u64>,
    #[serde(rename="s")]
    pub signature: Option<String>
}
#[derive(Serialize, Deserialize)]
pub struct ViewFileRequestOptions {
    pub is_consumable: bool,
    pub is_consumed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ViewFileRequestDocument {
    pub _id: ObjectId,
    pub request_id: ObjectId,
    pub file_id: ObjectId,
    pub options: Option<ViewFileRequestOptions>,
}

#[derive(Serialize, Deserialize)]
pub struct ViewFileRequest {
    pub request_id: ObjectId,
    pub file_id: ObjectId,
    pub options: Option<ViewFileRequestOptions>,
}

#[derive(Serialize, Deserialize)]
pub struct RequestWithBucketDocument {
    pub _id: ObjectId,
    #[serde(rename="project_id")]
    pub bucket_id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub permission: String,
    pub files: Option<Vec<String>>,
    pub options: Option<RequestOptions>,
    pub bucket: Option<BucketDocument>,
    pub target: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct UploadRequestWithBucketDocument {
    pub _id: ObjectId,
    pub date_created: u64,
    pub expiration_date: u64,
    pub permission: String,
    pub options: Option<RequestOptions>,
    pub bucket: BucketDocument,
    pub target: Option<String>
}

