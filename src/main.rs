
mod util;
mod handlers;
mod network;
mod actions;
mod models;
use std::format;
use dotenv::dotenv;
use network::App_Router;

#[tokio::main]
async fn main(){
    dotenv().ok();
    //routes
    let router = App_Router::Router().await;
    let address=std::env::var("ADDRESS").unwrap();
    let port = std::env::var("PORT").unwrap();
    println!("{}:{}", address, port );
    
    

    //let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}",address = address, port = port)).await.unwrap();
    println!("---> Listening to: {}", format!("{address}:{port}",address = address, port = port));
    axum::serve(listener, router).await.unwrap();
}
