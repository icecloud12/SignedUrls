
use mongodb::{results::InsertOneResult, Collection, Database, bson::doc};
use std::fs::{read, read_dir,create_dir};
use glob::glob;
use crate::network::{Db_Connection::DATABASE, DB_Collection};
use crate::models::Project::{ProjectModel, ProjectDocument};

pub async fn insert_project_if_exists( project_name:&String) -> Option<String>{
    let db= DATABASE.get().unwrap();

    //check if exists
    let if_exist_result = db.collection::<ProjectDocument>(DB_Collection::PROJECT.to_string().as_str()).find_one(doc! {
        "name": project_name.as_str()
    }, None).await;
    
    
    let x: Option<String> = match if_exist_result {
        Ok(o_project_model) => {
            let y:String = match o_project_model {
                Some(model) => {
                    model._id.to_string()
                },
                None => {
                    let doc= ProjectModel{
                        name: project_name.to_string()
                    };
                    let insert_one_result = db.collection::<ProjectModel>(DB_Collection::PROJECT.to_string().as_str()).insert_one(doc, None).await.unwrap();
                    println!("insert one result {:?}",insert_one_result);
                    insert_one_result.inserted_id.to_string()
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

pub async fn create_project_directory(project_id:&String) -> (bool, &str) {
    let path:String = format!("/projects/{}",project_id);
    let ret = match glob(path.as_str()) {
        Ok(_)=>{(false,"project already exists")},
        Err(_)=>{
            let create_result = create_dir(path);
            match create_result {
                Ok(_) => {(true, "successfully created directory")},
                Err(_) => ((false, "failed to create directory"))
            }
        }
    };
    return ret;

    //check if folder of project_id does not exist
    // if exist pass,
    // if none create
}