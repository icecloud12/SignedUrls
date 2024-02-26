use std;

use mongodb::{
    options::ClientOptions, Client, Database,
    Collection
    
};
use crate::network::DB_Collection;
use std::sync::OnceLock;
pub static DATABASE: OnceLock<Database> = OnceLock::new();


pub struct ProjectModel{
    name:String,
}
pub struct MongoCollection {
    project : OnceLock<Collection<ProjectModel>>
}
pub async fn Connect()-> Database{
    //database connection
    let options = ClientOptions::parse(std::env::var("DATABASE_URI").unwrap()).await.unwrap();
    let client = Client::with_options(options).unwrap();
    let database = client.database(std::env::var("DATABASE_NAME").unwrap().as_str());
    return database;
}

async fn Create_Collection(collection_name:&str)->Result<(),mongodb::error::Error>{

    let create_collection_request= DATABASE.get().unwrap().create_collection("project", None).await;
    return create_collection_request.clone()
}
