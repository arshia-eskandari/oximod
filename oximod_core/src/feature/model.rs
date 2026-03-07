use crate::error::oximod_error::OxiModError;
use crate::feature::conn::client::OxiClient;
use async_trait;
use mongodb::Client;
use mongodb::{
    Collection,
    bson::{Document, oid::ObjectId},
    results::DeleteResult,
};

/// Core asynchronous model interface for OxiMod-backed MongoDB documents.
///
/// This trait is typically implemented automatically via `#[derive(Model)]`.
/// It provides a minimal, typed API for:
///
/// - accessing a model's MongoDB collection,
/// - saving a model instance,
/// - clearing all documents from a model's collection,
/// - working either with an explicit [`mongodb::Client`] or the globally
///   initialized [`OxiClient`].
///
/// # Design
///
/// `Model` is intentionally lightweight. OxiMod provides schema-awareness,
/// builder-style ergonomics, and validation, while still exposing the underlying
/// MongoDB driver patterns when needed.
///
/// In practice, this means:
///
/// - use [`Model::save`] and [`Model::clear`] for common operations,
/// - use [`Model::get_collection`] when you want direct access to
///   `mongodb::Collection<Self>`,
/// - use [`Model::get_document_collection`] when you want a raw
///   `mongodb::Collection<Document>` for untyped operations such as
///   custom queries, ad hoc updates, or aggregation-oriented workflows.
///
/// # Global vs explicit client usage
///
/// The trait supports two access patterns:
///
/// ## 1. Global client
///
/// Methods like [`Model::save`], [`Model::clear`], and [`Model::get_collection`]
/// use the globally initialized [`OxiClient`].
///
/// This is convenient for application-wide access:
///
/// ```ignore
/// OxiClient::init_global("mongodb://localhost:27017").await?;
/// let id = user.save().await?;
/// ```
///
/// ## 2. Explicit client
///
/// Methods ending in `_from`, such as [`Model::save_from`] and
/// [`Model::clear_from`], operate on a caller-provided [`mongodb::Client`].
///
/// This is useful for:
///
/// - tests,
/// - scoped client lifetimes,
/// - multi-client or multi-database setups,
/// - avoiding reliance on global state.
///
/// ```ignore
/// let client = oxi_client.client().unwrap();
/// let id = user.save_from(client).await?;
/// ```
///
/// # Typed vs raw collections
///
/// [`Model::get_collection`] and [`Model::get_collection_from`] return
/// `Collection<Self>`, which is the preferred typed API.
///
/// [`Model::get_document_collection`] and
/// [`Model::get_document_collection_from`] return `Collection<Document>`,
/// which is useful when you need to work with raw BSON documents.
///
/// Prefer typed collections when possible:
///
/// ```ignore
/// let collection = User::get_collection()?;
/// let user = collection.find_one(doc! { "_id": id }).await?;
/// ```
///
/// Use document collections when you explicitly want raw BSON:
///
/// ```ignore
/// let collection = User::get_document_collection()?;
/// let doc = collection.find_one(doc! { "_id": id }).await?;
/// ```
///
/// # Implementors
///
/// This trait is generally not implemented manually. Instead, derive it:
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize)]
/// #[db("app_db")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     name: String,
///     age: i32,
/// }
/// ```
///
/// The derive macro is expected to generate the required collection resolution
/// and persistence behavior for the annotated model.
///
/// # Thread-safety
///
/// Implementors must be:
///
/// - [`Send`]
/// - [`Sync`]
/// - [`Sized`]
///
/// so model operations can safely participate in async workflows.
#[async_trait::async_trait]
pub trait Model
where
    Self: Send + Sync + Sized,
{
    /// Returns the typed MongoDB collection for this model using an explicit client.
    ///
    /// This is the fundamental collection accessor that implementors are expected
    /// to provide. It resolves the model's configured database and collection name
    /// and returns a typed `Collection<Self>`.
    ///
    /// Most users will prefer [`Model::get_collection`] unless they specifically
    /// need to work with an explicit client.
    ///
    /// # Parameters
    ///
    /// - `client`: A MongoDB client to use for resolving the collection.
    ///
    /// # Returns
    ///
    /// A typed [`mongodb::Collection`] whose document type is `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if the collection cannot be resolved.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let collection = User::get_collection_from(client)?;
    /// let found = collection.find_one(doc! { "name": "Alice" }).await?;
    /// ```
    fn get_collection_from(client: &mongodb::Client) -> Result<Collection<Self>, OxiModError>;

    /// Returns the raw BSON document collection for this model using an explicit client.
    ///
    /// This is a convenience wrapper around [`Model::get_collection_from`] that
    /// converts the typed collection into `Collection<Document>`.
    ///
    /// This is useful when you want to work directly with raw BSON documents
    /// instead of strongly typed model instances.
    ///
    /// # Parameters
    ///
    /// - `client`: A MongoDB client to use for resolving the collection.
    ///
    /// # Returns
    ///
    /// A raw [`mongodb::Collection`] of [`mongodb::bson::Document`].
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if the underlying typed collection cannot be resolved.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let collection = User::get_document_collection_from(client)?;
    /// let docs = collection.find(doc! { "active": true }).await?;
    /// ```
    fn get_document_collection_from(
        client: &mongodb::Client,
    ) -> Result<Collection<Document>, OxiModError> {
        Ok(Self::get_collection_from(client)?.clone_with_type::<Document>())
    }

    /// Persists this model instance using an explicit MongoDB client.
    ///
    /// Implementations are expected to serialize and insert `self` into the model's
    /// configured collection, returning the inserted document's [`ObjectId`].
    ///
    /// This method is the explicit-client counterpart to [`Model::save`].
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to use for the save operation.
    ///
    /// # Returns
    ///
    /// The inserted document's [`ObjectId`].
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if validation, serialization, collection access,
    /// or insertion fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id = user.save_from(client).await?;
    /// ```
    async fn save_from(&self, client: &mongodb::Client) -> Result<ObjectId, OxiModError>;

    /// Deletes all documents in this model's collection using an explicit client.
    ///
    /// This method removes every document in the collection associated with the model.
    /// It is primarily useful for:
    ///
    /// - tests,
    /// - examples,
    /// - resetting known datasets.
    ///
    /// This is the explicit-client counterpart to [`Model::clear`].
    ///
    /// # Parameters
    ///
    /// - `client`: The MongoDB client to use for the delete operation.
    ///
    /// # Returns
    ///
    /// A [`DeleteResult`] describing how many documents were removed.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection access or deletion fails.
    ///
    /// # Warning
    ///
    /// This removes **all documents** from the model's collection.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = User::clear_from(client).await?;
    /// println!("Deleted {}", result.deleted_count);
    /// ```
    async fn clear_from(client: &mongodb::Client) -> Result<DeleteResult, OxiModError>;

    /// Returns the typed MongoDB collection for this model using the global [`OxiClient`].
    ///
    /// This method retrieves the globally initialized client via [`OxiClient::global`]
    /// and delegates to [`Model::get_collection_from`].
    ///
    /// Use this when your application relies on a shared global client and you want
    /// direct access to `Collection<Self>`.
    ///
    /// # Returns
    ///
    /// A typed [`mongodb::Collection`] whose document type is `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - or the collection cannot be resolved.
    ///
    /// # Example
    ///
    /// ```ignore
    /// OxiClient::init_global("mongodb://localhost:27017").await?;
    /// let collection = User::get_collection()?;
    /// ```
    fn get_collection() -> Result<Collection<Self>, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::get_collection_from(client)
    }

    /// Returns the raw BSON document collection for this model using the global [`OxiClient`].
    ///
    /// This is a convenience wrapper around [`Model::get_collection`] that converts
    /// the typed collection into `Collection<Document>`.
    ///
    /// # Returns
    ///
    /// A raw [`mongodb::Collection`] of [`mongodb::bson::Document`].
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - or the collection cannot be resolved.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let collection = User::get_document_collection()?;
    /// let docs = collection.find(doc! {}).await?;
    /// ```
    fn get_document_collection() -> Result<Collection<Document>, OxiModError> {
        Ok(Self::get_collection()?.clone_with_type::<Document>())
    }

    /// Persists this model instance using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::save_from`].
    ///
    /// It is the most convenient way to save a model when your application uses
    /// a globally initialized MongoDB client.
    ///
    /// # Returns
    ///
    /// The inserted document's [`ObjectId`].
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - validation fails,
    /// - serialization fails,
    /// - collection resolution fails,
    /// - or insertion fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// OxiClient::init_global("mongodb://localhost:27017").await?;
    /// let id = user.save().await?;
    /// ```
    async fn save(&self) -> Result<ObjectId, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        self.save_from(client).await
    }

    /// Deletes all documents in this model's collection using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::clear_from`].
    ///
    /// # Returns
    ///
    /// A [`DeleteResult`] describing how many documents were removed.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or deletion fails.
    ///
    /// # Warning
    ///
    /// This removes **all documents** from the model's collection.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = User::clear().await?;
    /// println!("Deleted {}", result.deleted_count);
    /// ```
    async fn clear() -> Result<DeleteResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::clear_from(client).await
    }
}
