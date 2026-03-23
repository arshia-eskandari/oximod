use crate::error::oximod_error::OxiModError;
use async_trait::async_trait;
use mongodb::bson::{Document, oid::ObjectId};

/// Lifecycle hook interface for OxiMod-backed MongoDB models.
///
/// This trait is intended to complement [`Model`] by providing optional
/// lifecycle extension points around model-aware operations.
///
/// Hooks are designed to stay within OxiMod's schema-aware boundary.
/// They apply to model operations such as saving, updating, deleting,
/// and optionally finding documents by `_id`.
///
/// Hooks do **not** apply to raw collection access methods such as:
///
/// - `Model::get_collection`
/// - `Model::get_collection_from`
/// - `Model::get_document_collection`
/// - `Model::get_document_collection_from`
///
/// This preserves OxiMod's design philosophy:
///
/// > provide ergonomic, model-aware workflows without interfering with
/// > direct MongoDB driver usage.
///
/// # Default behavior
///
/// All hook methods have default no-op implementations that return `Ok(())`.
/// This means implementors may override only the hooks they care about.
///
/// # Error handling
///
/// Each hook returns `Result<(), OxiModError>`.
///
/// This allows hooks to:
///
/// - block an operation before it executes,
/// - perform validation or normalization,
/// - run side effects and surface failures consistently.
///
/// For pre-hooks, returning an error prevents the corresponding operation
/// from continuing.
///
/// For post-hooks, returning an error indicates that the underlying database
/// operation may already have succeeded, but post-processing failed.
///
/// # Intended usage
///
/// This trait is generally expected to be implemented automatically alongside
/// `#[derive(Model)]`, with users overriding hook methods as needed.
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize)]
/// #[db("app_db")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     email: String,
///     name: String,
/// }
///
/// #[async_trait::async_trait]
/// impl Hooks for User {
///     async fn pre_save(&mut self) -> Result<(), OxiModError> {
///         self.email = self.email.trim().to_lowercase();
///         Ok(())
///     }
/// }
/// ```
///
/// # Design notes
///
/// Hooks are intentionally limited to model-aware operations.
/// They are not intended to wrap every raw MongoDB action.
///
/// This keeps the feature:
///
/// - predictable,
/// - maintainable,
/// - aligned with OxiMod's lightweight philosophy.
#[async_trait]
pub trait Hooks
where
    Self: Send + Sync + Sized,
{
    /// Runs before this model instance is saved using [`Model::save`].
    ///
    /// This hook is invoked when the model is saved through the immutable
    /// save workflow. It allows validation, logging, or other side effects
    /// that do not require mutating the model.
    ///
    /// Typical uses:
    ///
    /// - checking business rules,
    /// - performing additional validation,
    /// - logging or auditing,
    /// - triggering external side effects.
    ///
    /// If you need to mutate the model before saving, use [`Hooks::pre_save_mut`]
    /// together with [`Model::save_mut`] instead.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the save operation may continue.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] to abort the save operation.
    async fn pre_save(&self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before this model instance is saved using [`Model::save_mut`].
    ///
    /// This hook is invoked when the model is saved through the mutable
    /// save workflow. It allows modifying the model before validation
    /// and persistence.
    ///
    /// Typical uses:
    ///
    /// - trimming or normalizing string fields,
    /// - lowercasing email addresses,
    /// - populating derived fields,
    /// - setting timestamps,
    /// - enforcing business rules that require mutation.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the save operation may continue.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] to abort the save operation.
    async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after this model instance has been saved using [`Model::save`].
    ///
    /// This hook is invoked after the immutable save workflow completes.
    ///
    /// Typical uses:
    ///
    /// - logging,
    /// - emitting events,
    /// - cache invalidation,
    /// - triggering follow-up actions.
    ///
    /// Note that by the time this hook runs, the underlying database
    /// operation has already completed.
    ///
    /// # Returns
    ///
    /// `Ok(())` if post-save processing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if post-save processing fails.
    async fn post_save(&self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after this model instance has been saved using [`Model::save_mut`].
    ///
    /// This hook is invoked after the mutable save workflow completes.
    ///
    /// This variant is useful when the save operation was performed with
    /// mutable access to the model and post-processing needs to observe
    /// the possibly modified state.
    ///
    /// Typical uses:
    ///
    /// - logging normalized values,
    /// - emitting events based on updated fields,
    /// - triggering follow-up actions that depend on mutations done in
    ///   [`Hooks::pre_save_mut`].
    ///
    /// Note that by the time this hook runs, the underlying database
    /// operation has already completed.
    ///
    /// # Returns
    ///
    /// `Ok(())` if post-save processing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if post-save processing fails.
    async fn post_save_mut(&self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is updated by its `_id`.
    ///
    /// This hook is invoked for model-aware update operations such as
    /// `Model::update_by_id` and `Model::update_by_id_from`.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to update.
    /// - `update`: The MongoDB update document that will be applied.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update operation may continue.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] to abort the update operation.
    async fn pre_update(_id: ObjectId, _update: &Document) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been updated by its `_id`.
    ///
    /// This hook is useful for post-update side effects such as logging,
    /// cache invalidation, or event emission.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the updated document.
    /// - `update`: The MongoDB update document that was applied.
    ///
    /// # Returns
    ///
    /// `Ok(())` if post-update processing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if post-update processing fails.
    ///
    /// Note that by the time this hook runs, the underlying update operation
    /// may already have completed successfully.
    async fn post_update(_id: ObjectId, _update: &Document) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is deleted by its `_id`.
    ///
    /// This hook is invoked for model-aware delete operations such as
    /// `Model::delete_by_id` and `Model::delete_by_id_from`.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to delete.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the delete operation may continue.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] to abort the delete operation.
    async fn pre_delete(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been deleted by its `_id`.
    ///
    /// This hook is useful for post-delete side effects such as logging
    /// or cache invalidation.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the deleted document.
    ///
    /// # Returns
    ///
    /// `Ok(())` if post-delete processing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if post-delete processing fails.
    ///
    /// Note that by the time this hook runs, the underlying delete operation
    /// may already have completed successfully.
    async fn post_delete(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is fetched by its `_id`.
    ///
    /// This is an optional read hook for model-aware find operations such as
    /// `Model::find_by_id` and `Model::find_by_id_from`.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document to find.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the find operation may continue.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] to abort the find operation.
    async fn pre_find(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been fetched by its `_id`.
    ///
    /// This is an optional read hook for model-aware find operations.
    ///
    /// # Parameters
    ///
    /// - `result`: The result returned by the find operation.
    ///
    /// # Returns
    ///
    /// `Ok(())` if post-find processing succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`OxiModError`] if post-find processing fails.
    async fn post_find(_result: &Option<Self>) -> Result<(), OxiModError> {
        Ok(())
    }
}
