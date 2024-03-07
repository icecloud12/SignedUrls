use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize)]
pub struct CreateSignaturePostRequestOptions{
    pub is_consumable: Option<bool>,
    
}
#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequest {
    pub project_name: Option<String>, //deserialization would throw an error and panics the program
    pub duration: Option<u64>, // defaults to env DEFAULT_DURATION variable
    pub is_consumable: Option<bool>,//defaults false
    pub target: Option<String> //target destination appended to the project-name as the path dir to upload
}

#[derive(Deserialize, Serialize)]
pub struct InsertRequest{
    pub project_name: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options:CreateSignaturePostRequestOptions,
    pub permission: String
}
#[derive(Deserialize, Serialize)]
pub struct UploadRequest {
    pub project_name: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options:CreateSignaturePostRequestOptions,
    pub permission: String,
    pub target: String
}


#[derive(Deserialize)]
pub struct RequestDocument {
    pub _id:ObjectId,
    pub project_name: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub permission: String
}