use mongodb::{results::InsertOneResult, Collection, Database, bson::doc};
use crate::network::{Db_Connection::DATABASE, DB_Collection};
use crate::models::Project::ProjectModel;

pub async fn insert_project_if_exists( project_name:String) //-> Result<InsertOneResult, mongodb::error::Error>
{
    let db= DATABASE.get().unwrap();

    //check if exists
    let if_exist_result = db.collection::<ProjectModel>(DB_Collection::PROJECT.to_string().as_str()).find_one(doc! {
        "name": project_name.as_str()
    }, None).await;
    
    /
    let x = match if_exist_result {
        Ok(o_project_model) => {
            //it exsits
            let ret = o_project_model.unwrap();
            return ret;
        },
        Err(error) => {
            //does not exist then insert
            let doc= ProjectModel{
                name: project_name
            };
            let insert_one_result = db.collection::<ProjectModel>(DB_Collection::PROJECT.to_string().as_str()).insert_one(doc, None).await.unwrap();
            insert_one_result.
            
        }
    };
    //create directory

    
}