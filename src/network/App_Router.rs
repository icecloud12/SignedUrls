use axum::{Router, routing::{get, post}};
use crate::handlers::{
    project_handler,
    //request_handler
};
pub async fn Router()->axum::Router {
    let router = Router::new()
        .route("/project/create", post(project_handler::create_project))
        //.route("/request/create", post(request_handler::create_request))
        ;
    return router;

}