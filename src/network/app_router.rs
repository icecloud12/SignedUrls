use std::env;

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
    let prefix = env::var("PREFIX").unwrap();

    let router = Router::new()
        //create the API_Key
        .route(format!("{}/project/create",prefix).as_str(), post(create_project))
        //CREATE uplaod/view requests
        .route(format!("{}/request/create/upload",prefix).as_str(), post(create_upload_request))
        .route(format!("{}/request/create/view",prefix).as_str(), post(create_view_request))
        //signed-url upload
        .route(format!("{}/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request))
        //signed-url-view
        .route(format!("{}/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),  get(process_signed_url_view_request))
        //public preview
        .route(format!("{}/preview/:file_id",prefix).as_str(), get(process_public_read_access))
        //upload view _KEY
        .route(format!("{}/direct/upload",prefix).as_str(), post(direct_upload))
      
        //no body limit
        .layer(DefaultBodyLimit::disable());
    return router;

}
