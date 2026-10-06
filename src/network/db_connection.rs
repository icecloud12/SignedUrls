use mongodb::{options::ClientOptions, Client, Database};

pub async fn connect(uri: &str, database_name: &str) -> mongodb::error::Result<Database> {
    let options = ClientOptions::parse(uri).await?;
    let client = Client::with_options(options)?;
    Ok(client.database(database_name))
}
