pub mod collections;
pub mod config;
pub mod error;
pub mod state;
pub mod tls;

mod models;
mod network;
mod project;
mod request;
mod signed_url;
mod util;

pub use config::Config;
pub use network::db_connection::connect;
pub use state::AppState;
pub use tls::rustls_config;

pub fn app(state: AppState) -> axum::Router {
    network::app_router::router(state)
}
