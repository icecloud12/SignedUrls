use argon2::{
    password_hash::{self, PasswordHasher, SaltString},
    Argon2,
};
use mongodb::{bson::doc, Database};
use std::fs;

use super::models::{ProjectDocument};
use crate::{network::{db_connection::DATABASE, DbCollection}, project::models::{BucketDocument, CreatedBucket, InsertBucketDocument}
};
use base64::{
    engine::general_purpose,
    Engine as _,
};
use rand::{self, RngCore};
pub static PUBLIC_KEY_LENGTH: usize = 16;
pub static SECRET_KEY_LENGTH: usize = 32;
pub async fn create_bucket(bucket_name: String) -> Result<CreatedBucket,()>{
    let db = DATABASE.get().unwrap();

    //check if exists
    let if_exist_result: Result<Option<BucketDocument>, mongodb::error::Error> = db
        .collection::<BucketDocument>(DbCollection::BUCKET.to_string().as_str())
        .find_one(
            doc! {
                "name": bucket_name.as_str()
            },
            None,
        )
        .await;

    match if_exist_result {
        Ok(o_bucket_document) => {
            match o_bucket_document {
                Some(_document) => Err(()),
                None => {
                    let public_key_raw: [u8; PUBLIC_KEY_LENGTH] =
                        generate_random_key::<PUBLIC_KEY_LENGTH>();
                    let secret_key_raw: [u8; SECRET_KEY_LENGTH] =
                        generate_random_key::<SECRET_KEY_LENGTH>();
                    let public_key_string = general_purpose::URL_SAFE_NO_PAD.encode(public_key_raw);
                    let secret_key_string = general_purpose::URL_SAFE_NO_PAD.encode(secret_key_raw);
                    let random_salt = SaltString::generate(&mut password_hash::rand_core::OsRng);
                    let argon2 = Argon2::default();
                    let secret_key_hash = argon2
                        .hash_password(&secret_key_raw, &random_salt)
                        .unwrap()
                        .to_string();
                    let secret_key_hint = {
                        let secret_key_string =
                            general_purpose::URL_SAFE_NO_PAD.encode(secret_key_raw);
                        let last_3: String = secret_key_string
                            .chars()
                            .skip(secret_key_string.len() - 3)
                            .take(3)
                            .collect();
                        "***".to_string() + &last_3
                    };
                    let doc = InsertBucketDocument {
                        name: &bucket_name,
                        public_key: &public_key_string,
                        hashed_secret_key: &secret_key_hash,
                        secret_key_hint: &secret_key_hint,
                    };
                    let insert_one_result = db
                        .collection::<InsertBucketDocument>(
                            DbCollection::BUCKET.to_string().as_str(),
                        )
                        .insert_one(doc, None)
                        .await
                        .unwrap();
                    let id = insert_one_result
                        .inserted_id
                        .as_object_id()
                        .unwrap()
                        .to_string();
                    // (id, api_key)
                    Ok(CreatedBucket {
                        _id: id,
                        name: bucket_name,
                        public_key: public_key_string,
                        secret_key: secret_key_string,
                        secret_key_hint: secret_key_hint,
                    })
                }
            }
            // Some(y)
            //return ret;
        }
        Err(_error) => { //something went wrong in fetching data
             Err(())
        }
    }
    // return x;
    //create directory
}

fn generate_random_key<const T: usize>() -> [u8; T] {
    let mut key = [0u8; T];
    // OsRng is the standard CSPRNG (Cryptographically Secure Pseudo-Random Number Generator)
    rand::rngs::OsRng.fill_bytes(&mut key);
    key
}

pub async fn create_bucket_directory(project_id: &String) {
    let path = std::path::PathBuf::from("./data").join(&project_id);
    if !(fs::metadata(&path).is_ok() && fs::metadata(&path).expect("Path does not exist").is_dir())
    {
        match std::fs::create_dir_all(path) {
            Ok(_) => {
                println!("created directory");
            }
            Err(error) => {
                println!("cannot create directory here");
                println!("{:?}", error)
            }
        }
    }
}

pub async fn get_project_id_by_name(project_name: String) -> Option<String> {
    let db: &Database = DATABASE.get().unwrap();
    let if_exist_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db
        .collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str())
        .find_one(doc! {"name":project_name.as_str()}, None)
        .await;

    let x: Option<String> = match if_exist_result {
        Ok(y) => {
            let z: Option<String> = match y {
                Some(a) => {
                    let id: String = a._id.to_string();
                    Some(id)
                }
                None => None,
            };
            z
        }
        Err(_error) => None,
    };
    return x;
}
pub async fn get_project_id_by_api_key(api_key: String) -> Option<String> {
    let db: &Database = DATABASE.get().unwrap();
    let if_exist_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db
        .collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str())
        .find_one(doc! {"api_key":api_key.as_str()}, None)
        .await;

    let x: Option<String> = match if_exist_result {
        Ok(y) => {
            let z: Option<String> = match y {
                Some(a) => {
                    let id: String = a._id.to_string();
                    Some(id)
                }
                None => None,
            };
            z
        }
        Err(_error) => None,
    };
    return x;
}
pub async fn validate_api_key(api_key: String) -> Option<ProjectDocument> {
    let db: &Database = DATABASE.get().unwrap();
    let project_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db
        .collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str())
        .find_one(
            doc! {
                signed_urls::collections::Bucket::PUBLIC_KEY : api_key
            },
            None,
        )
        .await;

    match project_result {
        Ok(project_option) => match project_option {
            Some(project) => {
                println!("project_found");
                Some(project)
            }
            None => {
                println!("project not found");
                None
            }
        },
        //can't do anything about mongodb internal error
        Err(err) => {
            println!("{}", err);
            None
        }
    }
}
