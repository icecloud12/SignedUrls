use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct ExtractSignedUrlParametersRequest{
    pub request_id: Option<ObjectId>,
    pub permission: Option<String>,
    pub created: Option<u64>,
    pub expiration: Option<u64>,
    pub nonce: Option<u64>,
    pub hashed_signature: Option<String>
}