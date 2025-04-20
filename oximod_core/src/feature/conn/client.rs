use std::sync::{Arc, OnceLock};
use mongodb::Client;
use crate::{error::oximod_error::OximodError, Printable, attach_printables};

static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

#[doc(hidden)]
/// Initializes a MongoDB client using the provided URI.
/// 
/// This is used internally by [`set_global_client`] to create the client.
/// 
/// # Arguments
/// * `mongo_uri` - A valid MongoDB connection string.
///
/// # Errors
/// Returns a [`OximodError::ConnectionError`] if the client initialization fails.
async fn init_db(mongo_uri: String) -> Result<Client, OximodError> {
    let client = Client::with_uri_str(&mongo_uri).await.map_err(|e|
        OximodError::ConnectionError(format!("{}", e))
    )?;

    Ok(client)
}

/// Retrieves the globally-initialized MongoDB client as an `Arc<Client>`.
///
/// This function must be called **after** [`set_global_client`] has been
/// successfully invoked. If not, it will return a [`OximodError::GlobalClientMissing`] error.
///
/// # Errors
/// Returns a [`OximodError::GlobalClientMissing`] if no client has been set.
pub fn get_global_client() -> Result<Arc<Client>, OximodError> {
    let client = CLIENT.get()
        .cloned()
        .ok_or_else(||
            attach_printables!(
                OximodError::GlobalClientMissing("Failed to clone arc".to_string()),
                "Ensure you call `set_global_client` before using `get_global_client`."
            )
        )?;
    Ok(client)
}

/// Sets the global MongoDB client used internally across the crate.
///
/// This function should be called **once**, typically at the start of your application.
/// It is used by the [`Model`] trait to access the MongoDB client.
///
/// # Arguments
/// * `mongo_uri` - A valid MongoDB connection string.
///
/// # Errors
/// - Returns [`OximodError::ConnectionError`] if the client cannot connect.
/// - Returns [`OximodError::GlobalClientInitError`] if called more than once.
pub async fn set_global_client(mongo_uri: String) -> Result<(), OximodError> {
    let client = init_db(mongo_uri).await?;

    CLIENT.set(client.into()).map_err(|_| 
        attach_printables!(
            OximodError::GlobalClientInitError("CLIENT set method failed.".to_string()),
            "Ensure `set_global_client` is only called once, or restart the application."
        )
    )?;

    Ok(())
}
