use mongodb::Database;

pub mod Db_Connection;
pub mod App_Router;

pub async fn Connect()-> Database{
    return Db_Connection::Connect().await;
}

pub enum DB_Collection {
    PROJECT,
}

impl ToString for DB_Collection {
    fn to_string(&self) -> String {
        match &self {
            &Self::PROJECT => "project".to_string(),
        }
    }
}