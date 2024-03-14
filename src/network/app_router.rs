use axum::{extract::DefaultBodyLimit, routing::{get, post}, Router};
use crate::project::handlers::create_project;
use crate::request::handlers::{
    create_upload_request, 
    create_view_request,
    process_public_read_access

};
use crate::signed_url::handlers::process_signed_url_upload_request;

pub async fn router()->axum::Router {
    let router = Router::new()
        .route("/project/create", post(create_project))
        .route("/request/create/upload", post(create_upload_request))
        .route("/request/create/view", post(create_view_request))
        //HANDLE THE SIGNED URL
        .route("/id/:project_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature", post(process_signed_url_upload_request))
        .route("/preview/:file_id", get(process_public_read_access)) //public preview
        .layer(DefaultBodyLimit::disable());
    return router;

}
