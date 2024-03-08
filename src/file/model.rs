
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct FileDocumentOptions{

}
#[derive(Deserialize, Serialize)]
pub struct FileDocument{
    pub _id: ObjectId,
    pub file_name: String,
    pub path: String,
    pub options: FileDocumentOptions
}

#[derive(Deserialize, Serialize)]
pub struct FileDocumentInsertRow {
    pub file_name: String,
    pub path: String,
    pub options: FileDocumentOptions
}