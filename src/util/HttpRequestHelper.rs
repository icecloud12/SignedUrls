// use std::net::TcpStream;
// struct Path {
//     domain: &str,
//     port: &str,
//     path: &str,
//     query: &str,
//     fragments: &str,

// }

use std::{
    net::TcpStream,
    io::{
        Result,
        prelude::*
    },
    str::*, clone
};

pub struct HTTPRequest {
    pub protocol: String,
    pub path: String,
    pub version: String, // ??
}

pub  fn wrap (line:String) -> HTTPRequest{
    let splitted_a:Vec<String>= line.split_whitespace().map(str::to_string).collect();
    
    HTTPRequest {
        protocol: splitted_a[0].to_owned(),
        path: splitted_a[1].to_owned(),
        version: splitted_a[2].to_owned(),
    }
}


enum Method {
    GET,
}