use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct ProjectModel{
//     pub name:String,
//     pub api_key:String,
// }
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectDocument {
    pub _id: ObjectId,
    pub name: String,
    pub api_key: String,
}
#[derive(Serialize)]
pub struct InsertBucketDocument<'a> {
    pub name: &'a String,
    pub public_key: &'a String,
    pub hashed_secret_key: &'a String,
    pub secret_key_hint: &'a String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct BucketDocument {
    pub _id: ObjectId,
    pub name: String,
    pub public_key: String,
    pub hashed_secret_key: Option<String>,
    pub secret_key_hint: Option<String>,
}

#[derive(Serialize)]
pub struct CreatedBucket {
    pub _id: String,
    pub name: String,
    pub public_key: String,
    pub secret_key: String,
    pub secret_key_hint: String,
}
