use std;

use mongodb::{
    options::ClientOptions, Client, Database
};
use std::sync::OnceLock;
pub static DATABASE: OnceLock<Database> = OnceLock::new();

pub async fn connect()-> Database{
    //database connection
    let options = ClientOptions::parse(std::env::var("DATABASE_URI").unwrap()).await.unwrap();
    let client = Client::with_options(options).unwrap();
    let database = client.database(std::env::var("DATABASE_NAME").unwrap().as_str());
    return database;
}
