use mongodb::bson::oid::ObjectId;

use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub struct RequestOptions {
    pub is_consumable: Option<bool>,
    pub is_consumed: Option<bool>,
    pub is_public: Option<bool>,
    pub size_limit: Option<usize>,
    pub mime_types: Option<Vec<String>>
}
#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequest {
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
    pub file_id_collection: Option<Vec<String>>,
    pub api_key: Option<String>,
    pub is_consumable: Option<bool>,
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

