use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize)]
pub struct CreateSignaturePostRequestOptions{
    pub is_consumable: Option<bool>,
    pub is_consumed: Option<bool>,
    pub is_public: Option<bool>
    
}
#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequest {
    //pub project_name: Option<String>, //deserialization would throw an error and panics the program (Deprecated and is moved to APIKEY headers)
    pub duration: Option<u64>, // defaults to env DEFAULT_DURATION variable
    pub target: Option<String>, //target destination appended to the project-name as the path dir to upload,
    pub is_consumable: Option<bool>,//defaults false
    pub is_public: Option<bool>, //when uploaded file becomes a public file where anybody can see
}



#[derive(Deserialize, Serialize)]
pub struct GenericRequest{
    pub project_id: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options:Option<CreateSignaturePostRequestOptions>,
    pub permission: String
}
#[derive(Deserialize, Serialize)]
pub struct UploadRequest {
    pub project_id: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options:CreateSignaturePostRequestOptions,
    pub permission: String,
    pub target: String
}


#[derive(Deserialize, Debug, Serialize)]
pub struct RequestDocument {
    pub _id:ObjectId,
    pub project_id: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub options:Option<RequestDocumentOptions>,
    pub permission: String
}

#[derive(Deserialize,Serialize, Debug)]
pub struct RequestDocumentOptions {
    pub is_consumable: bool,
    pub is_consumed: bool
}

//accepts a duration in seconds and a vector of file_ids
#[derive(Deserialize)]
pub struct CreateSignedUrlViewRequest{
    pub duration: Option<u64>,
    pub file_id_collection:Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct ViewRequest {
    pub project_id: String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub permission: String,
    pub files:Vec<String>,
    pub options: Option<CreateSignaturePostRequestOptions>
}
