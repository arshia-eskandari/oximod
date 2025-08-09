use crate::{attach_printables, error::oximod_error::OxiModError, Printable};
use mongodb::Client;
use std::sync::{Arc, OnceLock};

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
/// Returns a [`OxiModError::ConnectionError`] if the client initialization fails.
async fn init_db(mongo_uri: String) -> Result<Client, OxiModError> {
    let client = Client::with_uri_str(&mongo_uri)
        .await
        .map_err(|e| OxiModError::ConnectionError(format!("{}", e)))?;

    Ok(client)
}

/// Retrieves the globally-initialized MongoDB client as an `Arc<Client>`.
///
/// This function must be called **after** [`set_global_client`] has been
/// successfully invoked. If not, it will return a [`OxiModError::GlobalClientMissing`] error.
///
/// # Errors
/// Returns a [`OxiModError::GlobalClientMissing`] if no client has been set.
pub fn get_global_client() -> Result<Arc<Client>, OxiModError> {
    let client = CLIENT.get().cloned().ok_or_else(|| {
        attach_printables!(
            OxiModError::GlobalClientMissing("Failed to clone arc".to_string()),
            "Ensure you call `set_global_client` before using `get_global_client`."
        )
    })?;
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
/// - Returns [`OxiModError::ConnectionError`] if the client cannot connect.
/// - Returns [`OxiModError::GlobalClientInitError`] if called more than once.
pub async fn set_global_client(mongo_uri: String) -> Result<(), OxiModError> {
    let client = init_db(mongo_uri).await?;

    CLIENT.set(client.into()).map_err(|_| {
        attach_printables!(
            OxiModError::GlobalClientInitError("CLIENT set method failed.".to_string()),
            "Ensure `set_global_client` is only called once, or restart the application."
        )
    })?;

    Ok(())
}
