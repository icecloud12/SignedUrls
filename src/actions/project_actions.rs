
use mongodb::{ Database, bson::doc};
use std::fs;

use crate::network::{Db_Connection::DATABASE, DbCollection};
use crate::models::ProjectModel::{ProjectModel, ProjectDocument};

pub async fn insert_project_if_exists( project_name:&String) -> Option<String>{
    let db= DATABASE.get().unwrap();

    //check if exists
    let if_exist_result = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {
        "name": project_name.as_str()
    }, None).await;
    
    
    let x: Option<String> = match if_exist_result {
        Ok(o_project_model) => {
            let y:String = match o_project_model {
                Some(model) => {
                    let id = model._id.to_string();
                    id
                },
                None => {
                    let doc= ProjectModel{
                        name: project_name.to_string()
                    };
                    let insert_one_result = db.collection::<ProjectModel>(DbCollection::PROJECT.to_string().as_str()).insert_one(doc, None).await.unwrap();
                    println!("insert one result {:?}",insert_one_result);
                    let id = insert_one_result.inserted_id.as_object_id().unwrap().to_string();
                    id
                }
            };
            Some(y)
            //return ret;
        },
        Err(error) => { //something went wrong in fetching data
            None
        }
    };
    return x;
    //create directory

    
}

pub async fn create_project_directory(project_id:&String)  {
    let path = std::path::PathBuf::from("./data").join(&project_id);
    if !(fs::metadata(&path).is_ok() && fs::metadata(&path).expect("Path does not exist").is_dir()) {
        println!("directory does not exists. creating directory");
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
    let ifExistResult: Result<Option<ProjectDocument>, mongodb::error::Error> = db.collection::<ProjectDocument>(DbCollection::PROJECT.to_string().as_str()).find_one(doc! {"name":project_name.as_str()}, None).await;

    let x:Option<String> = match  ifExistResult {
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
        Err(error) => {
            None
        }
    };
    return x;

}