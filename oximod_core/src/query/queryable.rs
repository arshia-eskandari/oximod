//! Typed-query support for models.
//!
//! [`Queryable`] connects a model to the generated field structure used by
//! OxiMod's typed query, sorting, update, and deletion APIs.
//!
//! Applications normally receive this implementation through
//! `#[derive(Model)]` rather than implementing the trait manually.

use crate::query::builder::Query;

/// A model that supports OxiMod's typed query API.
///
/// Implementations provide an associated field structure containing one
/// typed [`Field`](crate::query::Field) for each model field.
///
/// OxiMod's `Model` derive implements this trait automatically.
///
/// # Example
///
/// ```ignore
/// use oximod::Model;
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     Model,
///     Serialize,
///     Deserialize,
///     Debug,
/// )]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     name: String,
///     age: i32,
///     active: bool,
/// }
///
/// let users = User::query()
///     .filter(|user| {
///         user.active.eq(true)
///             & user.age.gte(18)
///     })
///     .sort_by(|user| user.name.asc())
///     .all()
///     .await?;
/// ```
///
/// The closure passed to [`Query::filter`] receives the generated
/// [`Queryable::Fields`] value for the model. Its field methods are restricted
/// according to their Rust types, preventing incompatible MongoDB operations
/// from being constructed.
pub trait Queryable: Sized {
    /// The generated typed-field structure for this model.
    ///
    /// Each member represents a serialized MongoDB field path and exposes the
    /// query and update operations supported by its Rust type.
    ///
    /// For embedded documents, generated fields preserve the complete nested
    /// path.
    type Fields;

    /// Creates the generated field structure for this model.
    ///
    /// This method is used internally whenever OxiMod passes typed fields to a
    /// query, sorting, update, or array-filter closure.
    ///
    /// Applications normally access these fields through methods such as
    /// [`Query::filter`], [`Query::sort_by`], and [`Query::update_one`] rather
    /// than calling `fields()` directly.
    fn fields() -> Self::Fields;

    /// Creates an empty typed query for this model.
    ///
    /// The query can be configured with filters, sorting, pagination, text
    /// search, updates, or deletions before it is executed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let query = User::query()
    ///     .filter(|user| {
    ///         user.active.eq(true)
    ///     })
    ///     .limit(20);
    /// ```
    ///
    /// Creating a query does not communicate with MongoDB. An execution method
    /// such as [`Query::all`], [`Query::first`], [`Query::count`],
    /// [`Query::update_one`], or [`Query::delete_all`] must be called.
    fn query() -> Query<Self> {
        Query::new()
    }
}
