use mongodb::Database;

pub mod Db_Connection;
pub mod App_Router;

pub async fn Connect()-> Database{
    return Db_Connection::Connect().await;
}

pub enum DbCollection {
    PROJECT,
    REQUEST
}

impl ToString for DbCollection {
    fn to_string(&self) -> String {
        match &self {
            &Self::PROJECT => "project".to_string(),
            &Self::REQUEST => "request".to_string(),
        }
    }
}