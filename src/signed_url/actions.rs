use axum::{body::Bytes, Error};
use sha3::{Digest, Sha3_256};
use rand::{self, Rng};
use std::{
    path::Path,
    ffi::OsStr,
    time::{
        SystemTime, UNIX_EPOCH
    },
};


use tokio::{fs::File, io::AsyncWriteExt};
use crate::project;
pub enum ActionTypes {
    UPLOAD,
}

impl ToString for ActionTypes{
    fn to_string(&self)->String{
        match &self {
            &Self::UPLOAD => "upload".to_string()
        }
    }
}

pub struct CreateHashedSignatureResult{
    pub hashed_signature_base_64:String,
    pub date_created: u64,
    pub expiration_date: u64,
    pub nonce: u64
}
pub fn create_hashed_signature(
    project_id:&String,
    duration: &u64,
    action_type: &String,
)-> CreateHashedSignatureResult {
    let date_created:u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // let default_duration_offset: u64 = std::env::var("DEFAULT_DURATION").unwrap().parse::<u64>().unwrap();
    let expiration_date: u64 = date_created + duration;
    let nonce: u64 = rand::thread_rng().gen();
    let hashed_signature_base_64 = hash_parameters(&project_id, &date_created, &expiration_date, &action_type, &nonce);
    
    return CreateHashedSignatureResult {
        hashed_signature_base_64,
        date_created,
        expiration_date,
        nonce
    };
}
pub fn hash_parameters(
    project_id:&String,
    date_created: &u64,
    expiration_date: &u64,
    action_type: &String,
    nonce: &u64,
)-> String{
    let sk = std::env::var("SECRET_KEY").unwrap().to_string();
    let mut hasher = Sha3_256::new();
    println!("{:#?}|{:#?}|{:#?}|{:#?}|{:#?}|{:#?}",project_id,date_created,expiration_date,action_type,nonce, sk);
    hasher.update(&project_id.as_bytes());
    hasher.update((&date_created).to_be_bytes());
    hasher.update((&expiration_date).to_be_bytes());
    hasher.update(&action_type.as_bytes());
    hasher.update(&nonce.to_be_bytes());
    let signature = hasher.finalize();
    let hashed_signature_base_64 = signature.as_slice().iter().map(|b| format!("{:02x}", b)).collect::<String>();
    return hashed_signature_base_64;
}

pub async fn save_files_to_directory(
    file_names:Vec<String>,
    file_bytes: Vec<Bytes>,
    file_options: Vec<String>,

)-> Result<(), bool>{
    //the request itself should have a file path to go to
    for (index, element) in file_bytes.iter().enumerate(){
        let file_name = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();
        
        println!("fileNames: {}", file_names[index].as_str());
        let splits = file_names[index].as_str().split(".");
        let file_extention = splits.last().unwrap();
        println!("{}", file_extention);
        let mut file:File = File::create(format!("./data/{}.{}",file_name,file_extention)).await.unwrap();
        file.write(&element).await.unwrap();
    }
    Ok(())
}