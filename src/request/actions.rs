use hyper::StatusCode;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;


use crate::{file::model::FileDocument, network::DbCollection};
use crate::network::db_connection::DATABASE;
pub async fn file_reference_is_valid(file_id:ObjectId)->Result< FileDocument,(StatusCode, String)>{
    let db = DATABASE.get().unwrap();
    let file_ref:Result<Option<FileDocument>, mongodb::error::Error> = db.collection::<FileDocument>(DbCollection::FILE.to_string().as_str()).find_one(doc!{ "_id": file_id}, None).await;
    match file_ref {
        Ok(mongodb_ok)=>{
            match mongodb_ok {
                Some(file_doc)=>{
                    return Ok(file_doc);
                },
                None => return Err((StatusCode::NOT_FOUND, "file not found".to_string()))
            }

        },
        Err(_) =>{
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error")))
        }
        
    }
}