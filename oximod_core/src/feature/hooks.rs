//! Lifecycle hooks for collection-backed OxiMod models.
//!
//! Hooks extend the persistence helpers provided by
//! [`Model`](crate::feature::model::Model). They do not wrap typed-query
//! execution or direct operations performed through a MongoDB collection.

use crate::error::oximod_error::OxiModError;
use async_trait::async_trait;
use mongodb::bson::{Document, oid::ObjectId};

/// Lifecycle hook interface for collection-backed OxiMod models.
///
/// Hook implementations are enabled by combining `#[derive(Model)]` with the
/// `#[hooks]` attribute. Each method has a default no-op implementation, so a
/// model only needs to override the lifecycle events it uses.
///
/// Hooks run for the corresponding persistence helpers on
/// [`Model`](crate::feature::model::Model):
///
/// - `save`, `save_from`, and `save_with_session`;
/// - `save_mut`, `save_from_mut`, and `save_mut_with_session`;
/// - `find_by_id`, `find_by_id_from`, and `find_by_id_with_session`;
/// - `update_by_id`, `update_by_id_from`, and `update_by_id_with_session`;
/// - `delete_by_id`, `delete_by_id_from`, and `delete_by_id_with_session`.
///
/// Hooks do not run for:
///
/// - typed-query operations such as `query().first()`, `update_one()`, or
///   `delete_one()`;
/// - direct operations performed through a typed or raw MongoDB collection;
/// - collection accessors such as
///   [`Model::get_collection`](crate::feature::model::Model::get_collection)
///   and
///   [`Model::get_document_collection`](crate::feature::model::Model::get_document_collection).
///
/// Each save form runs only its own hooks: `save` and `save_from` run
/// [`Hooks::pre_save`]/[`Hooks::post_save`], while `save_mut` and
/// `save_from_mut` run [`Hooks::pre_save_mut`]/[`Hooks::post_save_mut`]. A
/// safeguard implemented in only one pre-save hook does not guard the other
/// save form; implement both when the application uses both.
///
/// # Sessions and transactions
///
/// Hook callbacks do not receive the caller's `ClientSession`. When a
/// `_with_session` helper fires hooks, the hooks run in the existing order,
/// but a database operation initiated inside a hook executes without the
/// session and is therefore **not** part of the caller's transaction. If a
/// side write must be atomic with the operation, perform it explicitly in
/// the transaction body with a session-aware operation instead of inside a
/// hook.
///
/// A post-hook for a session-aware helper runs after that MongoDB operation
/// succeeded in the session — not after the surrounding transaction
/// committed. OxiMod does not own transaction commit; an aborted transaction
/// rolls back the database operation even though its post-hook already ran.
///
/// # Error handling
///
/// Every hook returns `Result<(), OxiModError>`.
///
/// - An error from a pre-hook prevents the associated database operation.
/// - An error from a post-hook is returned after the database operation has
///   already succeeded.
///
/// # Example
///
/// ```ignore
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{Hooks, Model, OxiModError};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[db("app_db")]
/// #[collection("users")]
/// #[hooks]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     email: String,
///     name: String,
/// }
///
/// #[async_trait::async_trait]
/// impl Hooks for User {
///     async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
///         self.email = self.email.trim().to_lowercase();
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Hooks
where
    Self: Send + Sync + Sized,
{
    /// Runs before an immutable save operation.
    ///
    /// This hook runs before validation and insertion when the model is saved
    /// through [`Model::save`](crate::feature::model::Model::save) or
    /// [`Model::save_from`](crate::feature::model::Model::save_from).
    ///
    /// Use this hook for checks or side effects that do not mutate the model.
    /// To mutate the model before saving, use [`Hooks::pre_save_mut`].
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] aborts the save operation.
    async fn pre_save(&self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a mutable save operation.
    ///
    /// This hook runs before validation and insertion when the model is saved
    /// through [`Model::save_mut`](crate::feature::model::Model::save_mut) or
    /// [`Model::save_from_mut`](crate::feature::model::Model::save_from_mut).
    ///
    /// It can normalize values, populate derived fields, set timestamps, or
    /// perform other in-place preparation before persistence.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] aborts the save operation.
    async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after an immutable save operation succeeds.
    ///
    /// This hook runs after insertion when the model is saved through
    /// [`Model::save`](crate::feature::model::Model::save) or
    /// [`Model::save_from`](crate::feature::model::Model::save_from).
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] reports a post-processing failure. The
    /// document has already been inserted.
    async fn post_save(&self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a mutable save operation succeeds.
    ///
    /// This hook runs after insertion when the model is saved through
    /// [`Model::save_mut`](crate::feature::model::Model::save_mut) or
    /// [`Model::save_from_mut`](crate::feature::model::Model::save_from_mut).
    ///
    /// Mutations made here affect only the in-memory model. They are not
    /// persisted unless the model is saved again.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] reports a post-processing failure. The
    /// document has already been inserted.
    async fn post_save_mut(&mut self) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is updated by `_id`.
    ///
    /// This hook runs for
    /// [`Model::update_by_id`](crate::feature::model::Model::update_by_id) and
    /// [`Model::update_by_id_from`](crate::feature::model::Model::update_by_id_from).
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document being updated.
    /// - `update`: The MongoDB update document that will be applied.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] aborts the update operation.
    async fn pre_update(_id: ObjectId, _update: &Document) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been updated by `_id`.
    ///
    /// This hook runs after
    /// [`Model::update_by_id`](crate::feature::model::Model::update_by_id) or
    /// [`Model::update_by_id_from`](crate::feature::model::Model::update_by_id_from)
    /// succeeds.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the updated document.
    /// - `update`: The MongoDB update document that was applied.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] reports a post-processing failure. The update
    /// has already completed.
    async fn post_update(_id: ObjectId, _update: &Document) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is deleted by `_id`.
    ///
    /// This hook runs for
    /// [`Model::delete_by_id`](crate::feature::model::Model::delete_by_id) and
    /// [`Model::delete_by_id_from`](crate::feature::model::Model::delete_by_id_from).
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document being deleted.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] aborts the delete operation.
    async fn pre_delete(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been deleted by `_id`.
    ///
    /// This hook runs after
    /// [`Model::delete_by_id`](crate::feature::model::Model::delete_by_id) or
    /// [`Model::delete_by_id_from`](crate::feature::model::Model::delete_by_id_from)
    /// succeeds.
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the deleted document.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] reports a post-processing failure. The delete
    /// has already completed.
    async fn post_delete(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs before a document is fetched by `_id`.
    ///
    /// This hook runs for
    /// [`Model::find_by_id`](crate::feature::model::Model::find_by_id) and
    /// [`Model::find_by_id_from`](crate::feature::model::Model::find_by_id_from).
    ///
    /// # Parameters
    ///
    /// - `id`: The `_id` of the document being fetched.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] aborts the find operation.
    async fn pre_find(_id: ObjectId) -> Result<(), OxiModError> {
        Ok(())
    }

    /// Runs after a document has been fetched by `_id`.
    ///
    /// # Parameters
    ///
    /// - `result`: The model returned by the find operation, or `None` when no
    ///   matching document was found.
    ///
    /// # Errors
    ///
    /// Returning [`OxiModError`] reports a post-processing failure. The find
    /// operation has already completed.
    async fn post_find(_result: &Option<Self>) -> Result<(), OxiModError> {
        Ok(())
    }
}
