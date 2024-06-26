use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct File{
    pub _id:String,
    pub file_name:String, //original name,
    pub path:String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SaveFilesToDirectoryResult {
    pub request_id:String, //request_id reference it was created from
    pub path:String, //current path
    pub files: Vec<File>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DeleteFileUsingApiKey {
    pub api_key: Option<String>
}
