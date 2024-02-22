
mod util;
mod routes;
use axum::{response::Html, routing::{get, post}, Router, extract::Json};
use std::{
    format,
    env,
    error::Error
};
use dotenv::dotenv;
use mongodb::{Client, options::{ClientOptions, ResolverConfig}};
use util::collection::MongoDbCollection;
use routes::project_handler::{
        create_project,
        hello
    };
use serde::Deserialize;




#[tokio::main]
async fn main(){
    dotenv().ok();
    //routes
    let router = Router::new()
        .route("/hello", get(hello))
        .route("/project/create", post(create_project));

    ;
    let address=std::env::var("ADDRESS").unwrap();
    let port = std::env::var("PORT").unwrap();
    println!("{}:{}", address, port );
    
    //database connection
    let options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let client = Client::with_options(options).unwrap();

    //let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}",address = address, port = port)).await.unwrap();
    println!("---> Listening to: {}", format!("{address}:{port}",address = address, port = port));
    axum::serve(listener, router).await.unwrap();
}
