//! Runtime traits for collection-backed and embedded OxiMod models.
//!
//! The public [`Model`] trait exposes persistence operations for models backed
//! by their own MongoDB collections. Generated code uses the hidden
//! [`ModelCore`] trait and mode markers to share validation behavior with
//! models declared using `#[model(embedded)]`.

use crate::error::oximod_error::OxiModError;
use crate::feature::conn::client::OxiClient;
use async_trait::async_trait;
use mongodb::{
    Client, Collection as MongoCollection,
    bson::{Document, oid::ObjectId},
    results::{DeleteResult, UpdateResult},
};
use serde::{Serialize, de::DeserializeOwned};

/// Internal mode marker for a model backed by its own MongoDB collection.
///
/// This is the default mode used by [`ModelCore`]. Collection-backed models
/// also implement the public [`Model`] persistence trait.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Collection;

/// Internal mode marker for a model embedded inside another document.
///
/// Embedded models receive generated construction, fluent setters, defaults,
/// validation, and typed nested-field metadata, but they do not implement the
/// public [`Model`] persistence trait.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Embedded;

/// Internal marker implemented by OxiMod's supported model modes.
///
/// This trait is public only because it appears in the public bound on
/// [`ModelCore`]. It is intended for OxiMod-generated code.
#[doc(hidden)]
pub trait ModelMode: Send + Sync + 'static {}

impl ModelMode for Collection {}
impl ModelMode for Embedded {}

/// Internal interface shared by collection-backed and embedded models.
///
/// `#[derive(Model)]` implements this trait using one of two modes:
///
/// - [`Collection`] for ordinary models declared with `#[db(...)]` and
///   `#[collection(...)]`;
/// - [`Embedded`] for models declared with `#[model(embedded)]`.
///
/// The derive macro also generates an inherent `validate()` method, so
/// application code does not need to import this internal trait.
#[doc(hidden)]
pub trait ModelCore<M = Collection>: Send + Sync + Sized
where
    M: ModelMode,
{
    /// Runs the inline and user-defined validators generated for this model.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] when one or more configured validations fail.
    fn validate(&self) -> Result<(), OxiModError>;
}

/// Public persistence interface for collection-backed OxiMod models.
///
/// This trait is implemented automatically by `#[derive(Model)]` for ordinary
/// models declared with `#[db(...)]` and `#[collection(...)]`. Importing
/// `oximod::Model` brings its persistence methods into scope. Models declared
/// with `#[model(embedded)]` do not implement this trait.
///
/// The trait provides:
///
/// - typed and raw MongoDB collection access;
/// - global-client and explicit-client save operations;
/// - lookup, update, and deletion by `_id`;
/// - collection clearing;
/// - existence checks;
/// - document counting.
///
/// # Global and explicit clients
///
/// Methods such as [`Model::save`], [`Model::clear`], and
/// [`Model::find_by_id`] use the globally initialized [`OxiClient`]. Methods
/// ending in `_from`, such as [`Model::save_from`] and
/// [`Model::find_by_id_from`], use a caller-provided [`Client`].
///
/// # Typed and raw collections
///
/// [`Model::get_collection`] and [`Model::get_collection_from`] return
/// `MongoCollection<Self>`. [`Model::get_document_collection`] and
/// [`Model::get_document_collection_from`] return
/// `MongoCollection<Document>` for raw BSON operations.
///
/// # Example
///
/// ```ignore
/// use mongodb::bson::oid::ObjectId;
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     name: String,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// User::clear().await?;
///
/// let id = User::new()
///     .name("User1")
///     .save()
///     .await?;
///
/// let user = User::find_by_id(id).await?;
/// # let _ = user;
/// # Ok(())
/// # }
/// ```
///
/// # Embedded models
///
/// Embedded models use the same derive macro without receiving persistence
/// methods:
///
/// ```ignore
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[model(embedded)]
/// struct Address {
///     street: String,
/// }
///
/// let address = Address::new()
///     .street("13544 Cane St");
///
/// address.validate()?;
/// # Ok::<(), oximod::OxiModError>(())
/// ```
///
/// # Implementors
///
/// Applications generally should not implement this trait manually. Generated
/// collection-backed implementations also use the hidden
/// `ModelCore<Collection>` validation implementation.
///
/// Implementors must be [`Serialize`], [`DeserializeOwned`], [`Send`],
/// [`Sync`], and [`Sized`].
#[async_trait]
pub trait Model:
    ModelCore<Collection> + Serialize + DeserializeOwned + Send + Sync + Sized
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
    /// Returns [`OxiModError`] if the collection cannot be provided.
    fn get_collection_from(client: &Client) -> Result<MongoCollection<Self>, OxiModError>;

    /// Returns the raw BSON document collection for this model using an explicit client.
    ///
    /// This is a convenience wrapper around [`Model::get_collection_from`] that
    /// converts the typed collection into `MongoCollection<Document>`.
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
    /// Returns [`OxiModError`] if the underlying typed collection cannot be provided.
    fn get_document_collection_from(
        client: &Client,
    ) -> Result<MongoCollection<Document>, OxiModError> {
        Ok(Self::get_collection_from(client)?.clone_with_type::<Document>())
    }

    /// Persists this model instance using an explicit MongoDB client.
    ///
    /// Implementations are expected to serialize and insert `self` into the model's
    /// configured collection, and return the inserted document's [`ObjectId`].
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
    async fn save_from(&self, client: &Client) -> Result<ObjectId, OxiModError>;

    /// Persists this model instance using an explicit MongoDB client with mutable access.
    ///
    /// Implementations are expected to serialize and insert `self` into the model's
    /// configured collection, and return the inserted document's [`ObjectId`].
    ///
    /// This method is the explicit-client counterpart to [`Model::save_mut`].
    ///
    /// Unlike [`Model::save_from`], this method allows mutable access to the model
    /// before persistence. This enables lifecycle hooks such as
    /// [`crate::feature::hooks::Hooks::pre_save_mut`] and
    /// [`crate::feature::hooks::Hooks::post_save_mut`] to modify or observe
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
    async fn save_from_mut(&mut self, client: &Client) -> Result<ObjectId, OxiModError>;

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
    async fn clear_from(client: &Client) -> Result<DeleteResult, OxiModError>;

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
    async fn find_by_id_from(id: ObjectId, client: &Client) -> Result<Option<Self>, OxiModError>;

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
    async fn delete_by_id_from(id: ObjectId, client: &Client) -> Result<DeleteResult, OxiModError>;

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
    /// When documents written by older model versions may still be present,
    /// prefer `$set` on specific dotted field paths over rewriting whole
    /// embedded values or documents: a whole-value replacement writes only
    /// the current model shape and can drop fields the running code no
    /// longer declares.
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
        client: &Client,
    ) -> Result<UpdateResult, OxiModError>;

    /// Checks whether any document matching `filter` exists using an explicit client.
    ///
    /// This method is a document-level existence probe: it runs `find_one`
    /// against the raw document collection and never deserializes the matched
    /// document into `Self`. A matching document that cannot deserialize as
    /// this model therefore still returns `Ok(true)`, in agreement with
    /// [`Model::count_from`] over the same filter.
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
    async fn exists_from(filter: Document, client: &Client) -> Result<bool, OxiModError> {
        let collection = Self::get_document_collection_from(client)?;
        let found = collection
            .find_one(filter)
            .await
            .map_err(|error| OxiModError::database("Failed to check document existence", error))?;
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
    async fn count_from(filter: Document, client: &Client) -> Result<u64, OxiModError> {
        let collection = Self::get_collection_from(client)?;
        collection
            .count_documents(filter)
            .await
            .map_err(|error| OxiModError::database("Failed to count matching documents", error))
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
    fn get_collection() -> Result<MongoCollection<Self>, OxiModError> {
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
    fn get_document_collection() -> Result<MongoCollection<Document>, OxiModError> {
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
    /// [`crate::feature::hooks::Hooks::pre_save_mut`] and
    /// [`crate::feature::hooks::Hooks::post_save_mut`] to modify or observe
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
