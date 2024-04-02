use axum::{extract::DefaultBodyLimit, routing::{get, post}, Router};
use crate::project::handlers::create_project;
use crate::request::handlers::{
    create_upload_request, 
    create_view_request,
    process_public_read_access

};
use crate::signed_url::handlers::{
    process_signed_url_view_request,
    process_signed_url_upload_request,
    direct_upload
};

pub async fn router()->axum::Router {
    let router = Router::new()
        //create the API_Key
        .route("/project/create", post(create_project))
        //CREATE uplaod/view requests
        .route("/request/create/upload", post(create_upload_request))
        .route("/request/create/view", post(create_view_request))
        //signed-url upload
        .route("/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature", post(process_signed_url_upload_request))
        //signed-url-view
        .route("/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",  get(process_signed_url_view_request))
        //public preview
        .route("/preview/:file_id", get(process_public_read_access))
        //upload view API_KEY
        .route("/direct/upload", post(direct_upload))
        //no body limit
        .layer(DefaultBodyLimit::disable());
    return router;

}
