use hyper::StatusCode;
use mongodb::{bson::Bson, Database};

pub mod app_router;
pub mod db_connection;

pub async fn connect() -> Database {
    return db_connection::connect().await;
}

pub enum DbCollection {
    BUCKET,
    PROJECT,
    REQUEST,
    VIEW_FILE_REQUEST,
    FILE,
}

impl ToString for DbCollection {
    fn to_string(&self) -> String {
        match &self {
            &Self::BUCKET => "project".to_string(),
            &Self::PROJECT => "project".to_string(),
            &Self::REQUEST => "request".to_string(),
            &Self::VIEW_FILE_REQUEST => "view_file_request".to_string(),
            &Self::FILE => "file".to_string(),
        }
    }
}

impl From<DbCollection> for String {
    fn from(value: DbCollection) -> Self {
        value.to_string()
    }
}

impl From<DbCollection> for Bson{
    fn from(value: DbCollection) -> Self {
        Bson::String(value.to_string())
    }
}
