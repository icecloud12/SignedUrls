use std::env;

use axum::{extract::DefaultBodyLimit, routing::{delete, get, post}, Router};
use crate::{project::handlers::create_project, signed_url::handlers::delete_file_using_api_key};
use crate::request::handlers::{
    create_upload_request, 
    create_view_request,
    process_public_read_access

};
use crate::signed_url::handlers::{
    process_signed_url_view_request,
    process_signed_url_upload_request
};

pub async fn router()->axum::Router {
    let prefix = env::var("PREFIX").unwrap();

    let router = Router::new()
        //create the API_Key
        .route(format!("{}/project/create",prefix).as_str(), post(create_project))//Legal Alias
        .route(format!("{}/v1/project/create",prefix).as_str(), post(create_project))
        .route(format!("{}/v2/project/create", prefix).as_str(), post(create_project))//Formalization

        //CREATE view requests
        .route(format!("{}/request/create/view",prefix).as_str(), post(create_view_request))//Legal Alias
        .route(format!("{}/v1/request/create/view",prefix).as_str(), post(create_view_request))
        .route(format!("{}/v2/request/create/view",prefix).as_str(), post(create_view_request))//Formalization
        
        //CREATE upload requests
        .route(format!("{}/request/create/upload",prefix).as_str(), post(create_upload_request))//Legal Alias
        .route(format!("{}/v1/request/create/upload",prefix).as_str(), post(create_upload_request))
        .route(format!("{}/v2/request/create/upload",prefix).as_str(), post(create_upload_request))//Formalization

        //signed-url upload
        .route(format!("{}/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request))//Legal Alias
        .route(format!("{}/v1/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request))
        .route(format!("{}/v2/id/:request_id/permission/upload/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature",prefix).as_str(), post(process_signed_url_upload_request))//Formalization

        //signed-url-view
        .route(format!("{}/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),  get(process_signed_url_view_request))//Legal Alias
        .route(format!("{}/v1/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),  get(process_signed_url_view_request))
        .route(format!("{}/v2/id/:request_id/permission/view/created/:created/expiration/:expiration/nonce/:nonce/signature/:signature/file/:file_id",prefix).as_str(),  get(process_signed_url_view_request))//Formalization

        //public preview
        .route(format!("{}/preview/:file_id",prefix).as_str(), get(process_public_read_access))//Legal Alias
        .route(format!("{}/v1/preview/:file_id",prefix).as_str(), get(process_public_read_access))
        .route(format!("{}/v2/preview/:file_id",prefix).as_str(), get(process_public_read_access))//Formalization

        .route(format!("{}/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))//Legal Alias
        .route(format!("{}/v1/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))
        .route(format!("{}/v2/delete/:file_id", prefix).as_str(), delete(delete_file_using_api_key))//Formalization
		//no body limit
		.layer(DefaultBodyLimit::disable());
    return router;

}
