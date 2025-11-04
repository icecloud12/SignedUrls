use std::env;
use axum::{extract::DefaultBodyLimit, routing::{delete, get, post}, Router};
use crate::{
    project::handlers::create_bucket,
    request::{upload::{create_upload_request_v1, create_upload_request_v2}, view::{create_view_request_v1, create_view_request_v2, process_public_read_access}},
    signed_url::{handlers::delete_file_using_api_key,
    process_upload::{process_signed_url_upload_request_v1, process_signed_url_upload_request_v2},
    process_view::{process_signed_url_view_request_v1, process_signed_url_view_request_v2}}};

pub async fn router()->axum::Router {
    let prefix = env::var("PREFIX").unwrap();

    let router = Router::new()
        //create the API_Key
        .route(format!("{}/project/create",prefix).as_str(), post(create_bucket))//Legal Alias
        .route(format!("{}/v1/service",prefix).as_str(), post(create_bucket))
        .route(format!("{}/v2/bucket", prefix).as_str(), post(create_bucket))//Formalization

        //CREATE view requests
        .route(format!("{}/request/create/view",prefix).as_str(), post(create_view_request_v1))//Legal Alias
        .route(format!("{}/v1/request/view",prefix).as_str(), post(create_view_request_v1))
        .route(format!("{}/v2/request/view",prefix).as_str(), post(create_view_request_v2))//Formalization
        
        //CREATE upload requests
        .route(format!("{}/request/create/upload",prefix).as_str(), post(create_upload_request_v1))//Legal Alias
        .route(format!("{}/v1/request/upload",prefix).as_str(), post(create_upload_request_v1))
        .route(format!("{}/v2/request/upload",prefix).as_str(), post(create_upload_request_v2))//Formalization

        //signed-url-view consumer
        .route(format!("{}/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),
            get(process_signed_url_view_request_v1))//Legal Alias
        .route(format!("{}/v1/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),
            get(process_signed_url_view_request_v1))
        //used to read public and protected views
        .route(format!("{}/v2/file/:file_id",prefix).as_str(),
            get(process_signed_url_view_request_v2))
        
        //signed-url upload consumer
        .route(format!("{}/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request_v1))//Legal Alias
        .route(format!("{}/v1/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request_v1))
        .route(format!("{}/v2/upload",prefix).as_str(), post(process_signed_url_upload_request_v2))//Formalization
        
        //public preview
        .route(format!("{}/preview/:file_id",prefix).as_str(), get(process_public_read_access))//Legal Alias
        .route(format!("{}/v1/preview/:file_id",prefix).as_str(), get(process_public_read_access))

        .route(format!("{}/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))//Legal Alias
        .route(format!("{}/v1/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))
        .route(format!("{}/v2/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))//Formalization
		//no body limit
		.layer(DefaultBodyLimit::disable());
    return router;

}
