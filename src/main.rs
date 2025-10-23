mod util;
mod network;
mod project;
mod request;
mod signed_url;
mod file;

use std::{env, net::SocketAddr, path::PathBuf};
use axum_server::tls_rustls::RustlsConfig;
use dotenv::dotenv;
use network::{app_router, db_connection::DATABASE};
#[tokio::main]
async fn main(){
    dotenv().ok();
    let db = network::connect().await;
    let _ = DATABASE.set(db);
    //routes
    let router = app_router::router().await;

    let subsciber = tracing_subscriber::fmt().compact().with_file(true).with_line_number(true).with_target(true).finish();
    tracing::subscriber::set_global_default(subsciber).unwrap();
    
    match RustlsConfig::from_pem_file(
        PathBuf::from("./src/cert/")
            .join("localhost.crt"),
            PathBuf::from("./src/cert/")
            .join("localhost.pem")
    ).await {
        Ok(config)=>{
            tracing::info!("TLS Certificates found");
            let ip: Vec<u8> = env::var("ADDRESS").unwrap().split(".").into_iter().map(|x| x.parse::<u8>().unwrap()).collect::<Vec<u8>>();
            let socket_address: [u8; 4] = [ip[0],ip[1],ip[2],ip[3]];
            let addr = SocketAddr::from((socket_address, env::var("PORT").unwrap().parse::<u16>().unwrap()));
            let addr_s = &addr.to_string();
            tracing::info!("listening on https://{addr_s}");
            axum_server::bind_rustls(addr,config).serve(router.into_make_service()).await.unwrap()        
        }
        Err(_e)=>{ 
            tracing::error!("Missing certificaes");
        }
    }
    
}
