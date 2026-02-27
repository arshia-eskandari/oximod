use crate::error::oximod_error::OxiModError;
use mongodb::Client;
use std::sync::{Arc, OnceLock};

static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

/// A lightweight wrapper around a MongoDB [`Client`].
///
/// `OxiClient` can be used in two ways:
/// - As a **local context**, by holding an instance with its own inner `Client`.
/// - Indirectly, via a **global client** initialized with [`OxiClient::init_global`].
///
/// The `inner` field is optional so that an `OxiClient` can be constructed
/// before a client is initialized and then populated later via [`OxiClient::init_client`].
pub struct OxiClient {
    inner: Option<Client>,
}

impl OxiClient {
    /// Creates a new [`OxiClient`] by establishing a MongoDB connection.
    ///
    /// This constructor connects to MongoDB using the provided `url` and, on
    /// success, stores the resulting [`Client`] inside `OxiClient::inner`.
    ///
    /// # Arguments
    /// * `url` - A valid MongoDB connection string (e.g. `"mongodb://localhost:27017"`).
    ///
    /// # Errors
    /// Returns [`OxiModError::ConnectionError`] if a client cannot be created
    /// from the given URI.
    pub async fn new(url: String) -> Result<Self, OxiModError> {
        let client = Self::connect(url).await?;
        Ok(OxiClient {
            inner: Some(client),
        })
    }

    /// Initializes a MongoDB client using the provided URI.
    ///
    /// This is used internally by [`OxiClient::new`] and [`OxiClient::init_client`]
    /// to create the underlying [`Client`] instance.
    ///
    /// # Arguments
    /// * `mongo_uri` - A valid MongoDB connection string.
    ///
    /// # Errors
    /// Returns a [`OxiModError::ConnectionError`] if the client initialization fails.
    async fn connect(mongo_uri: String) -> Result<Client, OxiModError> {
        let client = Client::with_uri_str(&mongo_uri)
            .await
            .map_err(|e| OxiModError::ConnectionError(format!("{e}")))?;

        Ok(client)
    }

    /// (Re)initializes the inner MongoDB client for this [`OxiClient`] instance.
    ///
    /// This method is useful if you want to construct an `OxiClient` first and
    /// establish the connection later, or if you need to point this particular
    /// instance at a different MongoDB URI than the global client.
    ///
    /// Calling this method replaces any existing client stored in `inner`.
    ///
    /// # Arguments
    /// * `mongo_uri` - A valid MongoDB connection string.
    ///
    /// # Errors
    /// Returns [`OxiModError::ConnectionError`] if the client cannot connect.
    pub async fn init_client(&mut self, mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;

        self.inner = Some(client);

        Ok(())
    }

    /// Returns a mutable reference to the inner MongoDB client, if initialized.
    ///
    /// This is intended for advanced use cases where you need to mutate the
    /// underlying [`Client`] stored in this [`OxiClient`], for example to
    /// tweak driver-level options or perform low-level operations that OxiMod
    /// does not expose directly.
    ///
    /// If no client has been initialized yet (e.g. [`OxiClient::new`] or
    /// [`OxiClient::init_client`] has not been called), this will return a
    /// reference to `None`.
    ///
    /// # Panics
    /// This method does not panic, but callers should handle the `None` case.
    pub fn client_mut(&mut self) -> Option<&Client> {
        self.inner.as_ref()
    }

    /// Returns an immutable reference to the inner MongoDB client, if initialized.
    ///
    /// This allows read-only access to the underlying [`Client`] stored inside
    /// this [`OxiClient`]. It is useful for advanced use cases where you want
    /// to work directly with the MongoDB driver while still going through the
    /// same client OxiMod is using.
    ///
    /// If no client has been initialized yet (e.g. [`OxiClient::new`] or
    /// [`OxiClient::init_client`] has not been called), this will return a
    /// reference to `None`.
    ///
    /// # Panics
    /// This method does not panic, but callers should handle the `None` case.
    pub fn client(&self) -> Option<&Client> {
        self.inner.as_ref()
    }

    /// Sets the global MongoDB client used internally across the crate.
    ///
    /// This method should be called **once**, typically at the start of your
    /// application. It is used by the [`Model`] trait (and other OxiMod APIs)
    /// as the default client when no explicit [`OxiClient`] is provided.
    ///
    /// # Arguments
    /// * `mongo_uri` - A valid MongoDB connection string.
    ///
    /// # Errors
    /// - Returns [`OxiModError::ConnectionError`] if the client cannot connect.
    /// - Returns [`OxiModError::GlobalClientInitError`] if a global client has
    ///   already been set (i.e. this is called more than once).
    pub async fn init_global(mongo_uri: String) -> Result<(), OxiModError> {
        let client = Self::connect(mongo_uri).await?;

        CLIENT.set(client.into()).map_err(|_| {
            attach_printables!(
                OxiModError::GlobalClientInitError("CLIENT set method failed.".to_string()),
                "Ensure `init_global` is only called once, or restart the application."
            )
        })?;
        Ok(())
    }

    /// Retrieves the globally-initialized MongoDB client as an `Arc<Client>`.
    ///
    /// This function must be called **after** [`OxiClient::init_global`] has been
    /// successfully invoked. If not, it will return a
    /// [`OxiModError::GlobalClientMissing`] error.
    ///
    /// # Errors
    /// Returns a [`OxiModError::GlobalClientMissing`] if no global client has been set.
    pub fn global() -> Result<Arc<Client>, OxiModError> {
        let client = CLIENT.get().cloned().ok_or_else(|| {
            attach_printables!(
                OxiModError::GlobalClientMissing("Failed to clone arc".to_string()),
                "Ensure you call `init_global` before using `OxiClient::global`."
            )
        })?;
        Ok(client)
    }
}
