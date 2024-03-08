use mongodb::Database;

pub mod db_connection;
pub mod app_router;

pub async fn connect()-> Database{
    return db_connection::connect().await;
}

pub enum DbCollection {
    PROJECT,
    REQUEST,
    FILE,
}

impl ToString for DbCollection {
    fn to_string(&self) -> String {
        match &self {
            &Self::PROJECT => "project".to_string(),
            &Self::REQUEST => "request".to_string(),
            &Self::FILE => "file".to_string(),
        }
    }
}