
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct FileDocumentOptions{
    pub is_public: bool
}
#[derive(Deserialize, Serialize)]
pub struct FileDocument{
    pub _id: ObjectId,
    pub file_name: String,
    pub path: String,
    pub project_id: ObjectId,
    pub request_id:ObjectId,
    pub options: FileDocumentOptions,
}

#[derive(Deserialize, Serialize)]
pub struct FileDocumentInsertRow {
    pub file_name: String,
    pub path: String,
    pub project_id: ObjectId,
    pub request_id:ObjectId,
    pub options: FileDocumentOptions,

}