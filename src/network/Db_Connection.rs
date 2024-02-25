use std;

use mongodb::{
    options::ClientOptions, Client, Database,
    
};


pub async fn Connect()-> Database{
    //database connection
    let options = ClientOptions::parse(std::env::var("DATABASE_URI").unwrap()).await.unwrap();
    let client = Client::with_options(options).unwrap();
    let database = client.database(std::env::var("DATABASE_NAME").unwrap().as_str());
    return database;
}

async fn Create_Collection(database:&Database, collection_name:&str)->Result<(),mongodb::error::Error>{
    let create_collection_request= &database.create_collection("project", None).await;
    return create_collection_request.clone()
}