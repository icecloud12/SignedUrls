use axum::{Router, routing::post};
use crate::handlers::{
    project_handler,
    request_handler,
    signed_url_handler
};
pub async fn router()->axum::Router {
    let router = Router::new()
        .route("/project/create", post(project_handler::create_project))
        .route("/request/create", post(request_handler::create_request))
        //HANDLE THE SIGNED URL
        .route("/id/:project_id/permission/:permission/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature", post(signed_url_handler::process_signed_url_request))
        ;
    return router;

}