//! MongoDB client management for OxiMod.
//!
//! [`OxiClient`](crate::feature::conn::client::OxiClient) supports a globally
//! initialized MongoDB client for ordinary
//! model operations and instance-level clients for explicit-client workflows.

use crate::error::oximod_error::OxiModError;
use mongodb::Client;
use std::sync::{Arc, OnceLock};

static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

/// Wrapper around a MongoDB [`Client`].
///
/// `OxiClient` supports two usage patterns:
///
/// - a global client initialized once with [`OxiClient::init_global`];
/// - an instance-level client created with [`OxiClient::new`] or initialized
///   later with [`OxiClient::init_client`].
///
/// The global client is used by collection-backed model operations when no
/// explicit MongoDB client is supplied. Instance-level clients are useful for
/// tests, dependency injection, and applications that work with multiple
/// MongoDB deployments.
#[derive(Default)]
pub struct OxiClient {
    inner: Option<Client>,
}

impl OxiClient {
    /// Creates an initialized client wrapper from a MongoDB connection URI.
    ///
    /// # Parameters
    ///
    /// - `mongo_uri`: A MongoDB connection URI, such as
    ///   `"mongodb://localhost:27017"`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError::Connection`](crate::error::oximod_error::OxiModError::Connection)
    /// if the MongoDB driver cannot create a client from the supplied URI.
    pub async fn new(mongo_uri: String) -> Result<Self, OxiModError> {
        let client = Self::connect(mongo_uri).await?;

        Ok(Self {
            inner: Some(client),
        })
    }

    /// Creates a MongoDB driver client from a connection URI.
    ///
    /// This helper is shared by [`OxiClient::new`],
    /// [`OxiClient::init_client`], and [`OxiClient::init_global`].
    async fn connect(mongo_uri: String) -> Result<Client, OxiModError> {
        Client::with_uri_str(&mongo_uri).await.map_err(|error| {
            OxiModError::connection(
                "Unable to establish MongoDB client from provided URI",
                error,
            )
        })
    }

    /// Initializes or replaces this wrapper's MongoDB client.
    ///
    /// A wrapper created with [`OxiClient::default`] has no client until this
    /// method succeeds.
    ///
    /// # Parameters
    ///
    /// - `mongo_uri`: The MongoDB connection URI used to create the client.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError::Connection`](crate::error::oximod_error::OxiModError::Connection)
    /// if the MongoDB driver cannot create a client from the supplied URI.
    pub async fn init_client(&mut self, mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;
        self.inner = Some(client);

        Ok(())
    }

    /// Returns the wrapped MongoDB client, if initialized.
    #[must_use]
    pub fn client(&self) -> Option<&Client> {
        self.inner.as_ref()
    }

    /// Returns mutable access to the wrapped MongoDB client, if initialized.
    #[must_use]
    pub fn client_mut(&mut self) -> Option<&mut Client> {
        self.inner.as_mut()
    }

    /// Initializes the global MongoDB client.
    ///
    /// This method should normally be called once during application startup.
    /// Subsequent attempts to initialize the global client return an error;
    /// a repeated call does not replace the existing client.
    ///
    /// Handle the returned `Result` rather than discarding it. Global-client
    /// operations require initialization to have completed successfully;
    /// until it has, they fail with
    /// [`OxiModError::GlobalClientMissing`](crate::error::oximod_error::OxiModError::GlobalClientMissing).
    ///
    /// # Parameters
    ///
    /// - `mongo_uri`: The MongoDB connection URI used to create the client.
    ///
    /// # Errors
    ///
    /// Returns:
    ///
    /// - [`OxiModError::Connection`](crate::error::oximod_error::OxiModError::Connection)
    ///   if the MongoDB driver cannot create a client from the supplied URI;
    /// - [`OxiModError::GlobalClientInit`](crate::error::oximod_error::OxiModError::GlobalClientInit)
    ///   if the global client has already been initialized.
    pub async fn init_global(mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;

        CLIENT.set(client.into()).map_err(|_| {
            OxiModError::global_client_init("Global MongoDB client has already been initialized")
        })?;

        Ok(())
    }

    /// Returns the globally initialized MongoDB client.
    ///
    /// The returned [`Arc`] is a clone of the shared global handle.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError::GlobalClientMissing`](crate::error::oximod_error::OxiModError::GlobalClientMissing)
    /// if [`OxiClient::init_global`] has not completed successfully.
    pub fn global() -> Result<Arc<Client>, OxiModError> {
        CLIENT.get().cloned().ok_or_else(|| {
            OxiModError::global_client_missing("Global MongoDB client has not been initialized")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OxiClient;

    #[test]
    fn default_client_is_uninitialized() {
        let mut client = OxiClient::default();

        assert!(client.client().is_none());
        assert!(client.client_mut().is_none());
    }
}
