use mongodb::{results::InsertOneResult, Collection, Database, bson::doc};
use crate::network::{Db_Connection::DATABASE, DB_Collection};
use crate::models::Project::ProjectModel;

pub async fn create_project( project_name:String) //-> Result<InsertOneResult, mongodb::error::Error>
{
    let db= DATABASE.get().unwrap();

    //check if exists
    let if_exist_result = db.collection::<ProjectModel>(DB_Collection::PROJECT.to_string().as_str()).find_one(doc! {
        "name": project_name.as_str()
    }, None).await;
    
    match if_exist_result {
        Ok(o_project_model) => {
            //it exsits
        },
        Err(error) => {
            //does not exist then insert
            let doc= ProjectModel{
                name: project_name
            };
            db.collection::<ProjectModel>(DB_Collection::PROJECT.to_string().as_str()).insert_one(doc, None);

        }
    }
    //create directory

    
}