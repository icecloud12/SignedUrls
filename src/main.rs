mod util;

use axum::{response::{Html}, routing::get};
//use mongodb::{options::{ClientOptions, ResolverConfig}, Client, Collection};
use util::collection::MongoDbCollection::{self, SampleCollection};
use std::{
    io::{
        Result,
        prelude::*
    },
    net::TcpStream,
    net::TcpListener, 
    fs,
};
use axum_tut::ThreadPool;
use util::HttpRequestHelper::{
    *,
    HTTPRequest, 
    Router,
    self
};

const IP_ADDRESS:&str = "127.0.0.1";
const PORT:&str = "7878";

fn main(){
    
    let mut router: Router = Router::new();
    router.get("/", ||{
        
    });
    listen(router);
}
fn listen(router: Router){
    let address: String = format!("{IP_ADDRESS}:{PORT}");
    let listener: Result<TcpListener> = TcpListener::bind(address);
    let pool = ThreadPool::new(4);
    match listener {
        Ok(listener) => {
            for l in listener.incoming(){
                pool.execute(||{
                    let mut l_stream: TcpStream = l.unwrap();
                    handle_connection(l_stream)
                });
            }
        },
        Err(e) => {
            
            println!("{:?}",e );
        }
            
    }
}
fn handle_connection(mut stream:TcpStream){
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    let mut lines = buffer.lines();

    let http_request: HTTPRequest = HttpRequestHelper::wrap(lines.next().unwrap().unwrap());
    let protocol_collection:Vec<&str> = vec!["GET", "POST", "PUT", "PATCH"];

    let protocol = http_request.protocol;
    
    let status_code: &str;
    let status_message: &str;
    if(protocol_collection.contains(&protocol.as_ref())){
        //file_path = "index.html";
        status_code = "200";

        status_message = "OK";
    }
    else {
        // /file_path = "error.html";
        status_code = "404";
        status_message = "Resource Not Found"
    }
    //let file = fs::read_to_string(file_path).unwrap();
    let content = "Hello world";
    let response: String = format!("HTTP/1.1 {} {} \r\nContent-Length: {}\r\n\r\n{}",status_code, status_message, content.len(), content);
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
    
}