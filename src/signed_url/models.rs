use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct File{
    pub _id:String,
    pub request_id:String, //request_id reference it was created from
    pub path:String, //current path
    pub file_name:String, //original name,
    pub public_url:Option<String> //if file is public on creation, this field will be generated
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SaveFilesToDirectoryResult {
    pub request_id:String,
    pub files: Vec<File>
}