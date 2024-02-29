use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize)]
pub struct CreateSignaturePostRequestOptions{
    pub is_consumable: Option<bool>
}
#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequest {
    pub project_name: Option<String>, //deserialization would throw an error and panics the program
    pub duration: Option<u64>, // defaults 25200
    pub is_consumable: Option<bool> //defaults false
}

#[derive(Deserialize, Serialize)]
pub struct InsertRequest{
    pub project_name: String,
    pub date_created: u64,
    pub epiration_date: u64,
    pub options:CreateSignaturePostRequestOptions
}

#[derive(Deserialize)]
pub struct RequestDocument {
    pub _id:ObjectId,
    pub project_name: String,
    pub date_created: u64,
    pub epiration_date: u64,
}