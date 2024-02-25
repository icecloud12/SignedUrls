use axum::{Router, routing::{get, post}};
use crate::handlers::project_handler::{
    create_project
};
pub async fn Router()->axum::Router {
    let router = Router::new().
        route("/project/create", post(create_project));
    return router;

}