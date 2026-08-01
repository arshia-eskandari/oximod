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
/// Collection-backed models are produced by the normal `#[derive(Model)]`
/// form together with `#[db(...)]` and `#[collection(...)]` attributes.
/// They support shared generated model behavior and the persistence
/// operations exposed through the public [`Model`] trait.
///
/// This is the default mode parameter for [`ModelCore`].
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Collection;

/// Internal mode marker for a model embedded inside another document.
///
/// Embedded models are produced with `#[derive(Model)]` and
/// `#[model(embedded)]`. They support generated construction, fluent setters,
/// defaults, validation, and typed nested-field metadata, but they do not have
/// an independent MongoDB collection and therefore do not implement the
/// public [`Model`] trait.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Embedded;

/// Internal marker trait implemented by OxiMod's supported model modes.
///
/// This trait is public only because it appears in the bound on [`ModelCore`].
/// Applications should select [`Collection`] or [`Embedded`] rather than
/// implementing additional modes.
#[doc(hidden)]
pub trait ModelMode: Send + Sync + 'static {}

impl ModelMode for Collection {}
impl ModelMode for Embedded {}

/// Internal interface shared by collection-backed and embedded OxiMod models.
///
/// This trait is implemented automatically via `#[derive(Model)]`. Its mode
/// parameter records whether the generated type is backed by its own MongoDB
/// collection or can only be embedded inside another document:
///
/// - [`Collection`] is the default mode. A collection-backed model implements
///   both `ModelCore<Collection>` and [`Model`].
/// - [`Embedded`] is selected with `#[model(embedded)]`. An embedded model
///   implements `ModelCore<Embedded>` but does not implement [`Model`].
///
/// # Generated model capabilities
///
/// The `Model` derive generates the model-level behavior that is meaningful in
/// both modes, including:
///
/// - constructor and fluent-builder APIs,
/// - configured field defaults,
/// - inline and user-defined validation,
/// - Serde-aware typed-field metadata,
/// - typed nested queries and updates when the model is embedded in another
///   model.
///
/// Persistence is exposed through the public [`Model`] trait. This keeps
/// embedded models from receiving collection access, save operations, hooks,
/// index initialization, or other behavior that requires an independent
/// MongoDB collection.
///
/// # Collection-backed model
///
/// A normal model is collection-backed by default:
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
/// ```
///
/// The derive implements `ModelCore<Collection>` and [`Model`] for
/// `User`.
///
/// # Embedded model
///
/// Use `#[model(embedded)]` for a model that can only exist inside another
/// document:
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
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     address: Address,
/// }
///
/// let user = User::new(
///     Address::new("13544 Cane St".to_owned()),
/// );
/// ```
///
/// The derive implements `ModelCore<Embedded>` for `Address`. It does not
/// implement [`Model`], so `Address` cannot be queried, saved,
/// cleared, or otherwise persisted independently.
///
/// # Embedded-mode restrictions
///
/// Because an embedded model has no independent collection, the following
/// collection-specific attributes are invalid with `#[model(embedded)]`:
///
/// - `#[db(...)]`
/// - `#[collection(...)]`
/// - `#[hooks(...)]`
/// - `#[index(...)]`
/// - `#[document_id_setter_ident(...)]`
/// - `#[index_max_retries(...)]`
/// - `#[index_max_init_seconds(...)]`
///
/// The derive macro should reject these combinations with a targeted compile
/// error rather than silently ignoring them.
///
/// # Thread safety
///
/// Implementors must be [`Send`], [`Sync`], and [`Sized`] so generated model
/// behavior can safely participate in asynchronous workflows.
#[doc(hidden)]
pub trait ModelCore<M = Collection>: Send + Sync + Sized
where
    M: ModelMode,
{
    /// Performs validation on a model instance using inline checks and user-defined validators.
    ///
    /// This method is available to both collection-backed and embedded models.
    /// For embedded fields, parent-model validation can call the embedded
    /// model's validation implementation and prefix any resulting field paths.
    ///
    /// # Returns
    ///
    /// The unit value `()` if all validations pass.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if:
    ///
    /// - inline checks fail,
    /// - user-defined validators fail,
    /// - or recursive validation of an embedded model fails.
    fn validate(&self) -> Result<(), OxiModError>;
}

/// Public interface for collection-backed OxiMod models.
///
/// This trait is implemented automatically by `#[derive(Model)]` for models
/// that use the normal collection-backed mode. Importing `oximod::Model`
/// makes the generated persistence methods available. Types marked with
/// `#[model(embedded)]` do not implement this trait.
///
/// `Model` provides a typed API for:
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
/// `Model` contains only behavior that requires an independent
/// MongoDB collection. Shared model behavior, particularly validation, is
/// defined internally by [`ModelCore`].
///
/// OxiMod provides schema awareness, builder-style ergonomics, defaults, and
/// validation while still exposing the underlying MongoDB driver patterns when
/// needed.
///
/// In practice:
///
/// - use [`Model::save`] and [`Model::clear`] for common
///   persistence operations,
/// - use helpers like [`Model::find_by_id`],
///   [`Model::delete_by_id`], [`Model::update_by_id`],
///   [`Model::exists`], and [`Model::count`] for common
///   convenience workflows,
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
/// Methods like [`Model::save`], [`Model::clear`],
/// [`Model::get_collection`], [`Model::find_by_id`], and
/// [`Model::count`] use the globally initialized [`OxiClient`].
///
/// ## 2. Explicit client
///
/// Methods ending in `_from`, such as [`Model::save_from`] and
/// [`Model::find_by_id_from`], operate on a caller-provided
/// [`mongodb::Client`].
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
/// [`Model::get_collection`] and
/// [`Model::get_collection_from`] return `MongoCollection<Self>`,
/// which is the preferred typed API.
///
/// [`Model::get_document_collection`] and
/// [`Model::get_document_collection_from`] return
/// `MongoCollection<Document>`, which is useful when you need to work with raw
/// BSON documents.
///
/// # Implementors
///
/// This trait is generally not implemented manually. Instead, derive `Model`
/// in collection mode:
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
/// The generated implementation also uses the internal
/// `ModelCore<Collection>` implementation.
///
/// # Thread safety and serialization
///
/// Implementors must be:
///
/// - [`Send`]
/// - [`Sync`]
/// - [`Sized`]
/// - [`Serialize`]
/// - [`DeserializeOwned`]
///
/// so collection operations can serialize model values, deserialize typed
/// query results, and safely participate in asynchronous workflows.
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
    /// Returns [`OxiModError`] if the collection cannot be resolved.
    fn get_collection_from(client: &Client) -> Result<MongoCollection<Self>, OxiModError>;

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
        client: &Client,
    ) -> Result<MongoCollection<Document>, OxiModError> {
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
    async fn save_from(&self, client: &Client) -> Result<ObjectId, OxiModError>;

    /// Persists this model instance using an explicit MongoDB client with mutable access.
    ///
    /// Implementations are expected to serialize and insert `self` into the model's
    /// configured collection, returning the inserted document's [`ObjectId`].
    ///
    /// This method is the explicit-client counterpart to [`Model::save_mut`].
    ///
    /// Unlike [`Model::save_from`], this method allows mutable access to the model
    /// before persistence. This enables lifecycle hooks such as
    /// [`crate::feature::hooks::Hooks::pre_save_mut`] and [`crate::feature::hooks::Hooks::post_save_mut`] to modify or observe
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
    async fn exists_from(filter: Document, client: &Client) -> Result<bool, OxiModError> {
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
    async fn count_from(filter: Document, client: &Client) -> Result<u64, OxiModError> {
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
    /// [`crate::feature::hooks::Hooks::pre_save_mut`] and [`crate::feature::hooks::Hooks::post_save_mut`] to modify or observe
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
