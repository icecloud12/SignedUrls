mod util;

use axum::{response::{Html}, routing::get, Router};
use std::{
    format,
    env,
    error::Error
};
use mongodb::{Client, options::{ClientOptions, ResolverConfig}};
use util::collection::MongoDbCollection;
#[tokio::main]
async fn main(){
    //routes
    let router = Router::new()
        .route("/hello", get(|| async {
            Html("Hello World!")
        }));
    let address: &str="127.0.0.1";
    let port:i32 = 3002;
    
    
    //database connection
    let options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let client = Client::with_options(options).unwrap();

    //let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}",address = address, port = port)).await.unwrap();
    println!("---> Listening to: {}", format!("{address}:{port}",address = address, port = port));
    axum::serve(listener, router).await.unwrap();
}
