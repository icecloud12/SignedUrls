use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateSignedUrlPostRequest {
    pub project_name: String,
    pub duration: Option<u64>, // defaults 25200
    pub is_consumable: Option<bool> //defaults false
}