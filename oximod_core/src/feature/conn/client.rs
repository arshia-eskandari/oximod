use crate::error::oximod_error::OxiModError;
use mongodb::Client;
use std::sync::{Arc, OnceLock};

static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

/// Lightweight wrapper around a MongoDB [`Client`].
///
/// `OxiClient` supports two usage patterns:
///
/// - **Global client** initialized once with [`OxiClient::init_global`]
/// - **Local client** stored inside an instance created with [`OxiClient::new`]
///
/// The global client is used by OxiMod APIs when no explicit client is provided,
/// while local clients allow more control for tests, multi-tenant applications,
/// or dependency injection.
pub struct OxiClient {
    inner: Option<Client>,
}

impl OxiClient {
    /// Creates a new [`OxiClient`] by connecting to MongoDB.
    ///
    /// # Arguments
    /// * `url` — MongoDB connection string (for example `"mongodb://localhost:27017"`).
    ///
    /// # Errors
    /// Returns [`OxiModError::ConnectionError`] if the client cannot be created.
    pub async fn new(url: String) -> Result<Self, OxiModError> {
        let client = Self::connect(url).await?;
        Ok(OxiClient {
            inner: Some(client),
        })
    }

    /// Establishes a MongoDB connection.
    ///
    /// Used internally by [`OxiClient::new`] and [`OxiClient::init_client`].
    async fn connect(mongo_uri: String) -> Result<Client, OxiModError> {
        let client = Client::with_uri_str(&mongo_uri).await.map_err(|e| {
            OxiModError::connection("Unable to establish MongoDB client from provided URI", e)
        })?;

        Ok(client)
    }

    /// Initializes or replaces the inner MongoDB client for this instance.
    ///
    /// This allows creating an `OxiClient` first and connecting later,
    /// or switching the connection used by this instance.
    ///
    /// # Errors
    /// Returns [`OxiModError::ConnectionError`] if the client cannot connect.
    pub async fn init_client(&mut self, mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;
        self.inner = Some(client);
        Ok(())
    }

    /// Returns a reference to the inner MongoDB client, if initialized.
    ///
    /// Useful for advanced cases where direct access to the driver is needed.
    pub fn client(&self) -> Option<&Client> {
        self.inner.as_ref()
    }

    /// Returns a mutable reference to the inner MongoDB client, if initialized.
    ///
    /// This allows low-level customization of the underlying driver.
    pub fn client_mut(&mut self) -> Option<&Client> {
        self.inner.as_ref()
    }

    /// Initializes the global MongoDB client.
    ///
    /// This should typically be called once at application startup.
    ///
    /// OxiMod APIs will use the global client when no explicit
    /// [`OxiClient`] is provided.
    ///
    /// # Errors
    /// - [`OxiModError::ConnectionError`] if the client cannot connect
    /// - [`OxiModError::GlobalClientInitError`] if already initialized
    pub async fn init_global(mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;

        CLIENT.set(client.into()).map_err(|_| {
            OxiModError::global_client_init("Global MongoDB client has already been initialized")
        })?;

        Ok(())
    }

    /// Returns the global MongoDB client.
    ///
    /// [`OxiClient::init_global`] must be called before using this.
    ///
    /// # Errors
    /// Returns [`OxiModError::GlobalClientMissing`] if no global client exists.
    pub fn global() -> Result<Arc<Client>, OxiModError> {
        CLIENT.get().cloned().ok_or_else(|| {
            OxiModError::global_client_missing("Global MongoDB client has not been initialized")
        })
    }
}
