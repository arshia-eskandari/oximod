//! Typed-query support for collection-backed models.
//!
//! [`Queryable`] connects a collection model to the generated root field
//! structure used by OxiMod's typed query, sorting, update, and deletion APIs.
//!
//! Applications normally receive this implementation through
//! `#[derive(Model)]` rather than implementing the trait manually. Models
//! declared with `#[model(embedded)]` use
//! [`FieldSchema`](crate::query::FieldSchema) for nested fields instead and do
//! not create collection queries.

use crate::query::builder::Query;

/// A collection-backed model that supports OxiMod's typed-query API.
///
/// Implementations provide an associated root field structure containing one
/// typed [`Field`](crate::query::Field) for each serialized model field.
///
/// OxiMod's `Model` derive implements this trait automatically for models with
/// `#[db(...)]` and `#[collection(...)]` attributes.
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
/// [`Queryable::Fields`] value for the model. Each field exposes only the
/// query and update methods supported by its Rust type.
pub trait Queryable: Sized {
    /// The generated root typed-field structure for this collection model.
    ///
    /// Each member represents a serialized MongoDB field path and exposes the
    /// query and update operations supported by its Rust type.
    ///
    /// Fields containing embedded models provide their nested typed paths
    /// through methods such as [`crate::query::Field::nested`].
    type Fields;

    /// Creates the generated root field structure for this model.
    ///
    /// This method is used internally whenever OxiMod passes typed fields to a
    /// filter, sort, update, or array-filter closure.
    ///
    /// Applications normally access these fields through methods such as
    /// [`Query::filter`], [`Query::sort_by`], and [`Query::update_one`] rather
    /// than calling `fields()` directly.
    #[must_use]
    fn fields() -> Self::Fields;

    /// Creates an empty typed query for this collection model.
    ///
    /// The query can be configured with filters, sorting, pagination, text
    /// search, updates, or deletions before execution.
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
    #[must_use = "queries do nothing unless an execution method is called"]
    fn query() -> Query<Self> {
        Query::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Queryable;
    use crate::query::Field;
    use mongodb::bson::doc;

    struct User;

    struct UserFields {
        name: Field<String>,
        active: Field<bool>,
    }

    impl Queryable for User {
        type Fields = UserFields;

        fn fields() -> Self::Fields {
            UserFields {
                name: Field::new("name"),
                active: Field::new("active"),
            }
        }
    }

    #[test]
    fn queryable_exposes_generated_root_fields() {
        let fields = User::fields();

        assert_eq!(fields.name.name(), "name");
        assert_eq!(fields.active.name(), "active");
    }

    #[test]
    fn queryable_creates_an_empty_query() {
        assert_eq!(User::query().into_filter_document(), doc! {},);
    }

    #[test]
    fn queryable_fields_build_root_filter_paths() {
        let query = User::query().filter(|user| user.name.eq("User1") & user.active.eq(true));

        assert_eq!(
            query.into_filter_document(),
            doc! {
                "$and": [
                    {
                        "name": "User1",
                    },
                    {
                        "active": true,
                    },
                ],
            },
        );
    }
}
