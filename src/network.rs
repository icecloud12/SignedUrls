use mongodb::Database;

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
