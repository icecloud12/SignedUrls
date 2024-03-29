use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectModel{
    pub name:String,
    pub api_key:String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectDocument{
    pub _id:ObjectId,
    pub name:String,
    pub api_key:String,
}