use mongodb::{results::InsertOneResult, Collection, Database};
use crate::network;
use crate::models::Project::ProjectModel;

pub fn Collection(database: &Database) -> Collection<ProjectModel>{
    let collection = database.collection::<ProjectModel>(network::DB_Collection::PROJECT.to_string().as_str());
    return collection;
}
pub async fn Create(database: &Database, project_name:String) -> Result<InsertOneResult, mongodb::error::Error>{
    let collection = Collection(&database);
    let doc: ProjectModel = ProjectModel{
        name: project_name,
    };
    return collection.insert_one(doc, None).await; 
}