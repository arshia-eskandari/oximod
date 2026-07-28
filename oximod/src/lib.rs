//! # OxiMod
//!
//! Schema-aware MongoDB modeling for Rust.
//!
//! OxiMod is a lightweight modeling layer built on top of the official
//! MongoDB Rust driver. It provides builder-style model construction,
//! validation, defaults, index declarations, and optional lifecycle hooks,
//! while preserving direct access to the underlying driver when needed.
//!
//! ## Features
//!
//! - derive-based model definitions
//! - builder-style model construction
//! - validation and defaults
//! - index declarations
//! - optional lifecycle hooks
//! - global and explicit-client workflows
//! - typed and raw MongoDB collection access
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mongodb::bson::{doc, oid::ObjectId};
//! use oximod::{Model, OxiClient};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("my_app_db")]
//! #[collection("users")]
//! struct User {
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     _id: Option<ObjectId>,
//!
//!     #[index(unique, name = "email_idx")]
//!     #[validate(email)]
//!     email: String,
//!
//!     #[validate(min_length = 3, max_length = 32)]
//!     name: String,
//!
//!     #[validate(non_negative)]
//!     age: i32,
//!
//!     #[default(false)]
//!     active: bool,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     OxiClient::init_global("mongodb://localhost:27017".to_string()).await?;
//!
//!     User::clear().await?;
//!
//!     let user = User::new()
//!         .email("alice@example.com")
//!         .name("Alice")
//!         .age(30)
//!         .active(true);
//!
//!     let id = user.save().await?;
//!
//!     if let Some(found) = User::find_by_id(id).await? {
//!         println!("Found user: {}", found.name);
//!     }
//!
//!     let count = User::count(doc! {}).await?;
//!     println!("Total users: {}", count);
//!
//!     let collection = User::get_collection()?;
//!
//!     collection
//!         .update_one(
//!             doc! { "_id": id },
//!             doc! { "$set": { "active": false } },
//!         )
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! For more complete examples, see the
//! [`examples/`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples) directory.

// --- public API ---

/// Primary error type used by OxiMod.
///
/// This type is returned by model operations that fail due to validation,
/// hook execution, client initialization, or MongoDB driver errors.
pub use oximod_core::error::oximod_error::OxiModError;

/// Represents invalid typed-query configuration.
///
/// `QueryError` is used when a query cannot be executed because one or more
/// builder options are invalid.
///
/// Query errors are normally returned through [`OxiModError::Query`].
/// They can be inspected through pattern matching or with
/// [`OxiModError::query_error`].
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
///     Model,
///     OxiModError,
///     QueryError,
///     Queryable,
/// };
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
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
/// # async fn run() -> Result<(), OxiModError> {
/// let result = User::query()
///     .page(0, 10)
///     .all()
///     .await;
///
/// match result {
///     Err(OxiModError::Query(
///         QueryError::InvalidPageNumber { page },
///     )) => {
///         println!("Invalid page number: {page}");
///     }
///     Err(error) => return Err(error),
///     Ok(users) => println!("Found {} users", users.len()),
/// }
///
/// # Ok(())
/// # }
/// ```
pub use oximod_core::error::query_error::QueryError;

/// Represents a validation failure for a specific model field.
pub use oximod_core::error::oximod_error::ValidationError;

/// Represents one or more validation failures collected during model validation.
pub use oximod_core::error::oximod_error::ValidationErrors;

/// MongoDB client wrapper used by OxiMod.
///
/// `OxiClient` supports both global and explicit-client workflows.
///
/// Global usage:
///
/// ```rust,no_run
/// use oximod::OxiClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     OxiClient::init_global("mongodb://localhost:27017".to_string()).await?;
///     Ok(())
/// }
/// ```
///
/// Explicit usage:
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{Model, OxiClient};
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
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     OxiClient::init_global("mongodb://localhost:27017".to_string()).await?;
///
///     let user = User::new().name("Alice");
///     let _id = user.save().await?;
///
///     Ok(())
/// }
/// ```
pub use oximod_core::feature::conn::client::OxiClient;

/// Trait for defining lifecycle hooks on OxiMod models.
///
/// Hooks allow custom logic to run before and after save, update, delete,
/// and query operations.
///
/// Hooks are optional and must be enabled with `#[hooks]` on the model.
pub use oximod_core::feature::hooks::Hooks;

/// Core trait implemented by all OxiMod models.
///
/// This trait provides the primary model API, including persistence,
/// lookup, mutation, counting, existence checks, and access to both typed
/// and raw MongoDB collections.
///
/// It is implemented automatically by `#[derive(Model)]`.
pub use oximod_core::feature::model::Model;

/// Derive macro for defining OxiMod models.
///
/// This macro generates:
///
/// - builder methods
/// - model methods
/// - validation support
/// - default handling
/// - index initialization
/// - optional hook integration
///
/// # Example
///
/// ```rust
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     name: String,
/// }
/// ```
pub use oximod_macros::Model;

/// Trait implemented by models that support OxiMod's typed-query API.
///
/// This trait is implemented automatically by [`Model`]. Importing it brings
/// methods such as [`Queryable::query`] into scope.
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{Model, OxiModError, Queryable};
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
/// # async fn run() -> Result<(), OxiModError> {
/// let users = User::query()
///     .filter(|user| user.name.eq("Alice"))
///     .all()
///     .await?;
///
/// println!("Found {} users", users.len());
///
/// # Ok(())
/// # }
/// ```
///
/// # Filtering
///
/// Equality comparisons are available for fields whose values can be
/// represented as BSON:
///
/// ```rust,ignore
/// user.name.eq("Alice")
/// user.name.ne("Bob")
/// ```
///
/// Ordered fields support:
///
/// ```rust,ignore
/// user.age.gt(18)
/// user.age.gte(18)
/// user.age.lt(65)
/// user.age.lte(65)
/// ```
///
/// Multiple values can be matched or excluded with:
///
/// ```rust,ignore
/// user.role.in_values(["admin", "manager"])
/// user.role.not_in_values(["banned", "suspended"])
/// ```
///
/// # Logical expressions
///
/// Expressions can be combined with `&` for AND and `|` for OR:
///
/// ```rust,ignore
/// user.active.eq(true) & user.age.gte(18)
/// ```
///
/// ```rust,ignore
/// user.role.eq("admin") | user.role.eq("manager")
/// ```
///
/// Parentheses can be used to create nested expressions:
///
/// ```rust,ignore
/// user.active.eq(true)
///     & (
///         user.role.eq("admin")
///             | user.role.eq("manager")
///     )
/// ```
///
/// Rust does not allow overloading the `&&` and `||` operators, so typed
/// query expressions use `&` and `|`.
///
/// # Sorting
///
/// Sort by one field:
///
/// ```rust,ignore
/// User::query()
///     .sort_by(|user| user.age.desc())
/// ```
///
/// Add secondary sort fields with `then_sort_by`:
///
/// ```rust,ignore
/// User::query()
///     .sort_by(|user| user.age.desc())
///     .then_sort_by(|user| user.name.asc())
/// ```
///
/// # Limits and pagination
///
/// ```rust,ignore
/// User::query()
///     .skip(20)
///     .limit(10)
/// ```
///
/// Pagination is one-based:
///
/// ```rust,ignore
/// User::query()
///     .page(2, 10)
/// ```
///
/// Invalid pagination configuration is returned as a
/// [`QueryError`](crate::QueryError) when the query is executed.
///
/// # Null and missing fields
///
/// Existence checks are available for every field:
///
/// ```rust,ignore
/// user.nickname.exists()
/// user.nickname.not_exists()
/// ```
///
/// Optional fields also support strict null checks:
///
/// ```rust,ignore
/// user.nickname.is_null()
/// user.nickname.is_not_null()
/// ```
///
/// `is_null` matches a field that exists and contains BSON null. It does
/// not match a field that is missing from the document.
///
/// # Regular expressions
///
/// String fields support BSON regular-expression queries:
///
/// ```rust,ignore
/// user.name.matches_regex("^Ali")
/// ```
///
/// Typed options can be combined:
///
/// ```rust,ignore
/// use oximod::RegexOption;
///
/// user.name.matches_regex_with_options(
///     "^alice",
///     [RegexOption::CaseInsensitive],
/// )
/// ```
///
/// # Array fields
///
/// Array fields support element membership:
///
/// ```rust,ignore
/// user.tags.contains("rust")
/// ```
///
/// Match arrays containing every requested value:
///
/// ```rust,ignore
/// user.tags.contains_all(["rust", "mongodb"])
/// ```
///
/// Match an exact array length:
///
/// ```rust,ignore
/// user.tags.has_size(2)
/// ```
///
/// # Execution
///
/// Retrieve all matching models:
///
/// ```rust,ignore
/// User::query().all().await?
/// ```
///
/// Retrieve the first matching model:
///
/// ```rust,ignore
/// User::query().first().await?
/// ```
///
/// Count matching documents:
///
/// ```rust,ignore
/// User::query().count().await?
/// ```
///
/// # Embedded documents
///
/// Derive [`EmbeddedDocument`] for nested types and use `.nested()`
/// to access their generated typed fields.
///
/// Use `.elem_match_nested()` to create typed `$elemMatch` queries for
/// arrays of embedded documents.
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
///     EmbeddedDocument,
///     Model,
///     Queryable,
/// };
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     Debug,
///     Default,
///     Serialize,
///     Deserialize,
///     EmbeddedDocument,
/// )]
/// #[serde(rename_all = "camelCase")]
/// struct Address {
///     city_name: String,
///     active: bool,
/// }
///
/// #[derive(
///     Debug,
///     Serialize,
///     Deserialize,
///     Model,
/// )]
/// #[db("example")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///
///     name: String,
///
///     #[serde(skip_serializing_if = "Option::is_none")]
///     address: Option<Address>,
///
///     addresses: Vec<Address>,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let users = User::query()
///     .filter(|user| {
///         user.address.nested(|address| {
///             address.city_name.eq("City1")
///                 & address.active.eq(true)
///         })
///     })
///     .sort_by(|user| {
///         user.address.nested(|address| {
///             address.city_name.asc()
///         })
///     })
///     .all()
///     .await?;
///
/// let users_with_matching_address = User::query()
///     .filter(|user| {
///         user.addresses.elem_match_nested(|address| {
///             address.city_name.eq("City1")
///                 & address.active.eq(true)
///         })
///     })
///     .all()
///     .await?;
///
/// # let _ = users;
/// # let _ = users_with_matching_address;
/// # Ok(())
/// # }
/// ```
pub use oximod_core::query::Queryable;

/// An option that modifies MongoDB regular-expression matching.
///
/// Multiple options can be combined when calling
/// `matches_regex_with_options`.
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
///     Model,
///     OxiModError,
///     Queryable,
///     RegexOption,
/// };
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
/// # async fn run() -> Result<(), OxiModError> {
/// let users = User::query()
///     .filter(|user| {
///         user.name.matches_regex_with_options(
///             "^alice",
///             [RegexOption::CaseInsensitive],
///         )
///     })
///     .all()
///     .await?;
///
/// println!("Found {} users", users.len());
///
/// # Ok(())
/// # }
/// ```
pub use oximod_core::query::RegexOption;

/// A type-safe MongoDB update expression used by typed-query update operations.
///
/// Update expressions are normally created through methods on generated typed
/// fields, such as `.set()`, and passed to [`Queryable::update_one`].
///
/// # Example
///
/// ```rust,ignore
/// let updated_user = User::query()
///     .filter(|user| user.name.eq("User1"))
///     .update_one(|user| user.active.set(true))
///     .await?;
/// ```
pub use oximod_core::query::UpdateExpression;

/// Trait implemented by embedded documents that support
/// typed nested-field queries.
///
/// It is implemented automatically by
/// `#[derive(EmbeddedDocument)]`.
pub use oximod_core::query::EmbeddedDocument;

/// Derive macro for embedded documents used in typed
/// nested-field queries.
pub use oximod_macros::EmbeddedDocument;

// --- Internal API ---

#[doc(hidden)]
pub use async_trait as _async_trait;

#[doc(hidden)]
pub use futures_util as _futures_util;

#[doc(hidden)]
pub use mongodb as _mongodb;

#[doc(hidden)]
pub use oximod_core::feature as _feature;

#[doc(hidden)]
pub use oximod_core::helpers as _helpers;

#[doc(hidden)]
pub use regex as _regex; // removes the need of importing the trait

#[doc(hidden)]
pub mod _query {
    pub use oximod_core::query::{
        ElementExpression, ElementField, Expression, Field, OrderedQueryValue, Query,
        SortExpression, StringQueryValue,
    };
}
