mod util;
mod network;
mod project;
mod request;
mod signed_url;
mod file;


use std::format;
use dotenv::dotenv;
use network::{app_router, db_connection::DATABASE};

#[tokio::main]
async fn main(){
    dotenv().ok();
    let db = network::connect().await;
    let _ = DATABASE.set(db);
    //routes
    let router = app_router::router().await;
    let address=std::env::var("ADDRESS").unwrap();
    let port = std::env::var("PORT").unwrap();
    println!("{}:{}", address, port );
    


    //let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}",address = address, port = port)).await.unwrap();
    println!("---> Listening to: {}", format!("{address}:{port}",address = address, port = port));
    axum::serve(listener, router).await.unwrap();
}
