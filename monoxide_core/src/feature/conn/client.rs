use std::sync::{ Arc, OnceLock };
use mongodb::Client;
use crate::{ error::conn_error::MongoDbError, Printable };
use std::backtrace::Backtrace;

static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

async fn init_db(mongo_uri: String) -> Result<Client, MongoDbError> {
    let client = Client::with_uri_str(&mongo_uri).await.map_err(|e|
        MongoDbError::ConnectionError(format!("{}", e))
    )?;

    Ok(client)
}

pub fn get_global_client() -> Result<Arc<Client>, MongoDbError> {
    let client = CLIENT.get()
        .cloned()
        .ok_or_else(||
            MongoDbError::ClientNotFound("Failed to clone arc".to_string()).attach_printables(
                Backtrace::capture(),
                Some("Ensure you call `set_global_client` before using `get_global_client`.")
            )
        )?;
    Ok(client)
}

pub async fn set_global_client(mongo_uri: String) -> Result<(), MongoDbError> {
    let client = init_db(mongo_uri).await?;

    CLIENT.set(client.into()).map_err(|_|
        MongoDbError::SetClientError("CLIENT set method failed.".to_string()).attach_printables(
            Backtrace::capture(),
            Some("Ensure `set_global_client` is only called once, or restart the application.")
        )
    )?;

    Ok(())
}
