use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct FileDocumentOptions {
    pub is_public: bool,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct FileDocument {
    pub _id: ObjectId,
    pub file_name: String,
    pub path: String,
    pub project_id: ObjectId,
    pub request_id: ObjectId,
    pub options: FileDocumentOptions,
    pub mime_type: Option<String>,
    pub extension: Option<String>,
    pub file_size: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct FileDocumentInsertRow {
    pub file_name: String,
    pub path: String,
    pub project_id: ObjectId,
    pub request_id: ObjectId,
    pub options: FileDocumentOptions,
    pub mime_type: Option<String>,
    pub extension: Option<String>,
    pub file_size: Option<usize>,
}

#[derive(Serialize)]
pub struct FileIdUrlPair {
    pub id: String,
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<usize>,
}
