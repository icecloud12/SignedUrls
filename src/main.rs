use std::net::SocketAddr;

use axum::{response::{Html, IntoResponse}, routing::get, Json, Router};
use std::format;

#[tokio::main]
async fn main() {
    let routes_hello = Router::new().route("/hello", get(|| async {
        Html("Hello World!")
    }));
    let address: &str="127.0.0.1";
    let port:i32 = 3001;
    let listener = tokio::net::TcpListener::bind(format!("{address}{port}",address = address, port = port)).await.unwrap();
    println!("---->>> Listening on 0.0.0.0:3000\n");
    axum::serve(listener, routes_hello).await.unwrap();
}
