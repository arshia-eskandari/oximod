use crate::error::oximod_error::OxiModError;
use crate::feature::conn::client::OxiClient;
use async_trait;
use mongodb::Client;
use mongodb::{
    Collection,
    bson::{Document, doc, oid::ObjectId},
    results::{DeleteResult, UpdateResult},
};
use serde::de::DeserializeOwned;

/// Core async model interface for OxiMod-backed MongoDB documents.
///
/// This trait is typically implemented automatically via `#[derive(Model)]`.
/// Provides a minimal typed API for:
///
/// - accessing a model's MongoDB collection,
/// - saving a model instance,
/// - clearing all documents from a model's collection,
/// - querying documents by `_id`,
/// - deleting and updating documents by `_id`,
/// - checking document existence,
/// - counting documents,
/// - working either with an explicit [`mongodb::Client`] or the globally
///   initialized [`OxiClient`].
///
/// # Design
///
/// `Model` is intentionally lightweight. OxiMod provides schema-awareness,
/// builder-style ergonomics, and validation, while still exposing the underlying
/// MongoDB driver patterns when needed.
///
/// In practice:
///
/// - use [`Model::save`] and [`Model::clear`] for common persistence operations,
/// - use helpers like [`Model::find_by_id`], [`Model::delete_by_id`],
///   [`Model::update_by_id`], [`Model::exists`], and [`Model::count`] for
///   common convenience workflows,
/// - use [`Model::get_collection`] when you want direct access to
///   `mongodb::Collection<Self>`,
/// - use [`Model::get_document_collection`] when you want a raw
///   `mongodb::Collection<Document>` for untyped operations.
///
/// # Global vs explicit client usage
///
/// The trait supports two access patterns:
///
/// ## 1. Global client
///
/// Methods like [`Model::save`], [`Model::clear`], [`Model::get_collection`],
/// [`Model::find_by_id`], and [`Model::count`] use the globally initialized
/// [`OxiClient`].
///
/// ## 2. Explicit client
///
/// Methods ending in `_from`, such as [`Model::save_from`] and
/// [`Model::find_by_id_from`], operate on a caller-provided [`mongodb::Client`].
///
/// This is useful for:
///
/// - tests,
/// - scoped client lifetimes,
/// - multi-client or multi-database setups,
/// - avoiding reliance on global state.
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
/// # Implementors
///
/// This trait is generally not implemented manually. Instead, derive it:
///
/// ```ignore
/// use mongodb::bson::oid::ObjectId;
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
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
    Self: DeserializeOwned + Send + Sync + Sized,
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
    async fn save_from(&self, client: &mongodb::Client) -> Result<ObjectId, OxiModError>;

    /// Persists this model instance using an explicit MongoDB client with mutable access.
    ///
    /// Implementations are expected to serialize and insert `self` into the model's
    /// configured collection, returning the inserted document's [`ObjectId`].
    ///
    /// This method is the explicit-client counterpart to [`Model::save_mut`].
    ///
    /// Unlike [`Model::save_from`], this method allows mutable access to the model
    /// before persistence. This enables lifecycle hooks such as
    /// [`Hooks::pre_save_mut`] and [`Hooks::post_save_mut`] to modify or observe
    /// the model during the save workflow.
    ///
    /// This is useful when the save operation needs to perform normalization
    /// or other in-place changes before validation and insertion.
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
    /// hook execution, or insertion fails.
    async fn save_from_mut(&mut self, client: &mongodb::Client) -> Result<ObjectId, OxiModError>;

    /// Deletes all documents in this model's collection using an explicit client.
    ///
    /// This method removes every document in the collection associated with the model.
    /// It is primarily useful for tests, examples, and resetting known datasets.
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
    async fn clear_from(client: &mongodb::Client) -> Result<DeleteResult, OxiModError>;

    /// Finds a document by its `_id` using an explicit client.
    ///
    /// This is a convenience method for a very common query pattern:
    /// resolving a single typed document by its MongoDB `_id`.
    ///
    /// Internally, this is equivalent in behavior to:
    ///
    /// ```ignore
    /// let collection = User::get_collection_from(client)?;
    /// let found = collection.find_one(doc! { "_id": id }).await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to find.
    /// - `client`: The MongoDB client to use for the query.
    ///
    /// # Returns
    ///
    /// `Ok(Some(Self))` if a matching document is found, otherwise `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection resolution or the query fails.
    async fn find_by_id_from(
        id: ObjectId,
        client: &mongodb::Client,
    ) -> Result<Option<Self>, OxiModError>;

    /// Deletes a document by its `_id` using an explicit client.
    ///
    /// This method removes at most one document whose `_id` matches `id`.
    ///
    /// Internally, this is equivalent in behavior to:
    ///
    /// ```ignore
    /// let collection = User::get_collection_from(client)?;
    /// let result = collection.delete_one(doc! { "_id": id }).await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to delete.
    /// - `client`: The MongoDB client to use for the delete operation.
    ///
    /// # Returns
    ///
    /// A [`DeleteResult`] indicating whether a document was deleted.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection resolution or deletion fails.
    async fn delete_by_id_from(
        id: ObjectId,
        client: &mongodb::Client,
    ) -> Result<DeleteResult, OxiModError>;

    /// Updates a document by its `_id` using an explicit client.
    ///
    /// This method updates at most one document whose `_id` matches `id`.
    ///
    /// The `update` document must follow MongoDB update syntax, such as:
    ///
    /// ```ignore
    /// doc! { "$set": { "active": false } }
    /// ```
    ///
    /// Internally, this is equivalent in behavior to:
    ///
    /// ```ignore
    /// let collection = User::get_collection_from(client)?;
    /// let result = collection.update_one(doc! { "_id": id }, update).await?;
    /// ```
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to update.
    /// - `update`: A MongoDB update document.
    /// - `client`: The MongoDB client to use for the update operation.
    ///
    /// # Returns
    ///
    /// An [`UpdateResult`] indicating how many documents matched and were modified.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection resolution or update execution fails.
    async fn update_by_id_from(
        id: ObjectId,
        update: Document,
        client: &mongodb::Client,
    ) -> Result<UpdateResult, OxiModError>;

    /// Checks whether any document matching `filter` exists using an explicit client.
    ///
    /// This method is implemented using `find_one(filter).await?.is_some()`,
    /// which is typically more efficient for existence checks than counting
    /// all matching documents.
    ///
    /// # Parameters
    ///
    /// - `filter`: A MongoDB filter document.
    /// - `client`: The MongoDB client to use for the query.
    ///
    /// # Returns
    ///
    /// `true` if at least one matching document exists, otherwise `false`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection resolution or the query fails.
    async fn exists_from(filter: Document, client: &mongodb::Client) -> Result<bool, OxiModError> {
        let collection = Self::get_collection_from(client)?;
        let found = collection
            .find_one(filter)
            .await
            .map_err(|e| OxiModError::database("Failed to check document existence", e))?;
        Ok(found.is_some())
    }

    /// Counts documents matching `filter` using an explicit client.
    ///
    /// This is a convenience wrapper around MongoDB's `count_documents`.
    ///
    /// # Parameters
    ///
    /// - `filter`: A MongoDB filter document.
    /// - `client`: The MongoDB client to use for the count operation.
    ///
    /// # Returns
    ///
    /// The number of matching documents.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if collection resolution or the count operation fails.
    async fn count_from(filter: Document, client: &mongodb::Client) -> Result<u64, OxiModError> {
        let collection = Self::get_collection_from(client)?;
        collection
            .count_documents(filter)
            .await
            .map_err(|e| OxiModError::database("Failed to count matching documents", e))
    }

    /// Returns the typed MongoDB collection for this model using the global [`OxiClient`].
    ///
    /// This method retrieves the globally initialized client via [`OxiClient::global`]
    /// and delegates to [`Model::get_collection_from`].
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
    fn get_document_collection() -> Result<Collection<Document>, OxiModError> {
        Ok(Self::get_collection()?.clone_with_type::<Document>())
    }

    /// Persists this model instance using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::save_from`].
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
    async fn save(&self) -> Result<ObjectId, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        self.save_from(client).await
    }

    /// Persists this model instance using the global [`OxiClient`] with mutable access.
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::save_from_mut`].
    ///
    /// Unlike [`Model::save`], this method allows mutable access to the model
    /// before persistence. This enables lifecycle hooks such as
    /// [`Hooks::pre_save_mut`] and [`Hooks::post_save_mut`] to modify or observe
    /// the model during the save workflow.
    ///
    /// This is useful when the save operation needs to perform normalization
    /// or other in-place changes before validation and insertion.
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
    /// - hook execution fails,
    /// - validation fails,
    /// - serialization fails,
    /// - collection resolution fails,
    /// - or insertion fails.
    async fn save_mut(&mut self) -> Result<ObjectId, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        self.save_from_mut(client).await
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
    async fn clear() -> Result<DeleteResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::clear_from(client).await
    }

    /// Finds a document by its `_id` using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::find_by_id_from`].
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to find.
    ///
    /// # Returns
    ///
    /// `Ok(Some(Self))` if a matching document is found, otherwise `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or the query fails.
    async fn find_by_id(id: ObjectId) -> Result<Option<Self>, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::find_by_id_from(id, client).await
    }

    /// Deletes a document by its `_id` using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::delete_by_id_from`].
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to delete.
    ///
    /// # Returns
    ///
    /// A [`DeleteResult`] indicating whether a document was deleted.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or deletion fails.
    async fn delete_by_id(id: ObjectId) -> Result<DeleteResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::delete_by_id_from(id, client).await
    }

    /// Updates a document by its `_id` using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::update_by_id_from`].
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to update.
    /// - `update`: A MongoDB update document.
    ///
    /// # Returns
    ///
    /// An [`UpdateResult`] indicating how many documents matched and were modified.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or update execution fails.
    async fn update_by_id(id: ObjectId, update: Document) -> Result<UpdateResult, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::update_by_id_from(id, update, client).await
    }

    /// Checks whether any document matching `filter` exists using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::exists_from`].
    ///
    /// # Parameters
    ///
    /// - `filter`: A MongoDB filter document.
    ///
    /// # Returns
    ///
    /// `true` if at least one matching document exists, otherwise `false`.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or the query fails.
    async fn exists(filter: Document) -> Result<bool, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::exists_from(filter, client).await
    }

    /// Counts documents matching `filter` using the global [`OxiClient`].
    ///
    /// This method retrieves the global client via [`OxiClient::global`] and
    /// delegates to [`Model::count_from`].
    ///
    /// # Parameters
    ///
    /// - `filter`: A MongoDB filter document.
    ///
    /// # Returns
    ///
    /// The number of matching documents.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - the global client has not been initialized,
    /// - collection resolution fails,
    /// - or the count operation fails.
    async fn count(filter: Document) -> Result<u64, OxiModError> {
        let client_arc = OxiClient::global()?;
        let client: &Client = client_arc.as_ref();
        Self::count_from(filter, client).await
    }
}
