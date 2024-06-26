
use hyper::HeaderMap;
use mongodb::{ Database, bson::doc};
use std::fs;

use crate::network::{db_connection::DATABASE, DbCollection};
use super::models::{ProjectDocument, ProjectModel};
use rand::{self, distributions::Alphanumeric, Rng};

pub async fn insert_project_if_exists( project_name:&String) -> Option<(String, String)>{
    let db= DATABASE.get().unwrap();

    //check if exists
    let if_exist_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {
        "name": project_name.as_str()
    }, None).await;
    
    
    let x: Option<(String,String)> = match if_exist_result {
        Ok(o_project_model) => {
            let y:(String,String) = match o_project_model {
                Some(model) => {
                    let id = model._id.to_string();
                    let api_key = model.api_key;
                    (id, api_key)
                },
                None => {
                    let api_key:String = rand::thread_rng().sample_iter(Alphanumeric).take(256).map(char::from).collect();
                    let doc= ProjectModel{
                        name: project_name.to_string(),
                        api_key: api_key.clone()
                    };
                    let insert_one_result = db.collection::<ProjectModel>(DbCollection::PROJECT.to_string().as_str()).insert_one(doc, None).await.unwrap();
                    let id = insert_one_result.inserted_id.as_object_id().unwrap().to_string();
                    (id, api_key)
                }
            };
            Some(y)
            //return ret;
        },
        Err(_error) => { //something went wrong in fetching data
            None
        }
    };
    return x;
    //create directory

    
}

pub async fn create_project_directory(project_id:&String)  {
    let path = std::path::PathBuf::from("./data").join(&project_id);
    if !(fs::metadata(&path).is_ok() && fs::metadata(&path).expect("Path does not exist").is_dir()) {
        match std::fs::create_dir_all(path) {
            Ok(_)=>{
                println!("created directory");
            },
            Err(error) => {
                println!("cannot create directory here");
                println!("{:?}",error)
            }
        }
    }
}

pub async fn get_project_id_by_name(project_name:String) -> Option<String>{
    let db: &Database = DATABASE.get().unwrap();
    let if_exist_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {"name":project_name.as_str()}, None).await;

    let x:Option<String> = match  if_exist_result {
        Ok(y)=>{
            let z:Option<String> = match y {
                Some(a) => {
                    let id:String = a._id.to_string();
                    Some(id)
                },
                None => {
                    None
                }
            };
            z
        },
        Err(_error) => {
            None
        }
    };
    return x;

}
pub async fn get_project_id_by_api_key(api_key:String) -> Option<String>{
    let db: &Database = DATABASE.get().unwrap();
    let if_exist_result: Result<Option<ProjectDocument>, mongodb::error::Error> = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {"api_key":api_key.as_str()}, None).await;

    let x:Option<String> = match  if_exist_result {
        Ok(y)=>{
            let z:Option<String> = match y {
                Some(a) => {
                    let id:String = a._id.to_string();
                    Some(id)
                },
                None => {
                    None
                }
            };
            z
        },
        Err(_error) => {
            None
        }
    };
    return x;

}
pub async fn validate_api_key(api_key: String)->Option<ProjectDocument>
{
    println!("api key sent:{}", api_key);
    let db:&Database = DATABASE.get().unwrap();
    let project_result:Result<Option<ProjectDocument>, mongodb::error::Error> = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one( doc!{
        "api_key": api_key
    }, None).await;

    match project_result {
        Ok(project_option)=>{
            match project_option {
                Some(project) =>{
                    println!("project_found");
                    Some(project)
                },
                None=> {
                    println!("project not found");
                    None
                }
                    
            }
        },
        //can't do anything about mongodb internal error
        Err(err)=> {
            println!("{}",err);
            None
        }
    }
        
       

}