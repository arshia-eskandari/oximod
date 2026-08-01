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
//! - derive-based collection and embedded model definitions
//! - builder-style model construction
//! - validation and defaults
//! - index declarations
//! - optional lifecycle hooks
//! - global and explicit-client workflows
//! - typed and raw MongoDB collection access
//! - type-safe filtering, sorting, pagination, updates, and deletions
//! - typed text-search and GeoJSON geospatial queries
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

/// A typed-query operation that may modify multiple documents.
pub use oximod_core::error::query_error::BulkWriteOperation;

/// An unsupported query modifier associated with a typed bulk write.
pub use oximod_core::error::query_error::QueryModifier;

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

/// Trait for defining lifecycle hooks on collection-backed OxiMod models.
///
/// Hooks allow custom logic to run before and after save, update, delete,
/// and query operations.
///
/// Hooks are optional and must be enabled with `#[hooks]` on a
/// collection-backed model. Embedded models do not support persistence hooks.
pub use oximod_core::feature::hooks::Hooks;

/// Public trait implemented by collection-backed OxiMod models.
///
/// Importing `oximod::Model` brings the derive macro and this trait into scope.
/// Rust keeps derive macros and traits in separate namespaces, so one import
/// supports both model declaration and collection persistence.
///
/// This trait provides:
///
/// - typed and raw MongoDB collection access,
/// - global and explicit-client save operations,
/// - lookup, update, and deletion by `_id`,
/// - collection clearing,
/// - existence checks,
/// - document counting.
///
/// It is implemented automatically for ordinary `#[derive(Model)]` types.
/// Models declared with `#[model(embedded)]` do not implement this trait.
/// Embedded models still receive generated builders, defaults, validation, and
/// typed nested-field metadata.
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
///     Model,
///     OxiModError,
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
/// let user = User::new().name("Alice");
/// let id = user.save().await?;
/// let found = User::find_by_id(id).await?;
///
/// # let _ = found;
/// # Ok(())
/// # }
/// ```
pub use oximod_core::feature::model::Model;

/// Derive macro for defining collection-backed and embedded OxiMod models.
///
/// By default, `#[derive(Model)]` generates a collection-backed model with:
///
/// - fluent builder methods,
/// - default handling,
/// - validation support,
/// - typed-query support,
/// - index initialization,
/// - optional hook integration,
/// - MongoDB collection and persistence support.
///
/// Collection-backed models require `#[db(...)]` and `#[collection(...)]`.
///
/// Use `#[model(embedded)]` to generate an embedded model. Embedded models
/// receive fluent builder methods, default handling, validation support, and
/// typed nested-field access, but do not receive collection access, querying,
/// indexes, hooks, or persistence methods.
///
/// # Collection-backed model
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
///     address: Address,
/// }
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[model(embedded)]
/// struct Address {
///     city: String,
/// }
/// ```
///
/// # Embedded model
///
/// ```rust
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[model(embedded)]
/// struct Address {
///     street: String,
///     city: String,
/// }
///
/// let address = Address::new()
///     .street("13544 Cane St")
///     .city("City1");
/// ```
///
/// The following attributes are not supported on embedded models:
///
/// - `#[db(...)]`
/// - `#[collection(...)]`
/// - `#[hooks]`
/// - `#[index(...)]`
/// - `#[document_id_setter_ident(...)]`
/// - `#[index_max_retries(...)]`
/// - `#[index_max_init_seconds(...)]`
pub use oximod_macros::Model;

/// Trait implemented by models that support OxiMod's typed-query API.
///
/// This trait is implemented automatically by `#[derive(Model)]`. Importing it
/// brings [`Queryable::query`] into scope.
///
/// A typed query receives generated model fields whose available operations
/// depend on their Rust types. This prevents incompatible MongoDB operators
/// from being applied to fields.
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
///     Model,
///     OxiModError,
///     Queryable,
/// };
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     Debug,
///     Serialize,
///     Deserialize,
///     Model,
/// )]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///
///     name: String,
///     age: i32,
///     active: bool,
///     role: String,
/// }
///
/// # async fn run() -> Result<(), OxiModError> {
/// let users = User::query()
///     .filter(|user| {
///         user.active.eq(true)
///             & user.age.gte(18)
///     })
///     .sort_by(|user| user.name.asc())
///     .limit(20)
///     .all()
///     .await?;
///
/// println!("Found {} users", users.len());
///
/// # Ok(())
/// # }
/// ```
///
/// # Equality and membership
///
/// Fields whose values can be represented as BSON support equality and
/// inequality:
///
/// ```rust,ignore
/// user.name.eq("User1")
/// user.name.ne("User2")
/// ```
///
/// Match one of several values with `$in`:
///
/// ```rust,ignore
/// user.role.in_values([
///     "admin",
///     "member",
/// ])
/// ```
///
/// Exclude several values with `$nin`:
///
/// ```rust,ignore
/// user.role.not_in_values([
///     "banned",
///     "suspended",
/// ])
/// ```
///
/// # Ordered comparisons
///
/// Numeric values, strings, and BSON date-time values support ordered
/// comparisons:
///
/// ```rust,ignore
/// user.age.gt(18)
/// user.age.gte(18)
/// user.age.lt(65)
/// user.age.lte(65)
/// ```
///
/// # Logical expressions
///
/// Combine expressions with `&` for MongoDB `$and` and `|` for `$or`:
///
/// ```rust,ignore
/// user.active.eq(true)
///     & (
///         user.role.eq("admin")
///             | user.role.eq("member")
///     )
/// ```
///
/// Rust does not allow overloading `&&` and `||`, so typed expressions use
/// the bitwise operators `&` and `|`.
///
/// Negate a field condition with `.not()`:
///
/// ```rust,ignore
/// user.age.not(|age| age.gte(18))
/// ```
///
/// # Field existence, null, and BSON type
///
/// Existence checks are available for every field:
///
/// ```rust,ignore
/// user.nickname.exists()
/// user.nickname.not_exists()
/// ```
///
/// Optional fields support strict null checks:
///
/// ```rust,ignore
/// user.nickname.is_null()
/// user.nickname.is_not_null()
/// ```
///
/// `is_null()` matches only fields that exist and contain BSON null. It does
/// not match missing fields.
///
/// Query the stored BSON representation with [`BsonType`]:
///
/// ```rust,ignore
/// user.nickname.has_bson_type(
///     BsonType::String,
/// )
/// ```
///
/// # Regular expressions
///
/// Required and optional string fields support BSON regular-expression
/// queries:
///
/// ```rust,ignore
/// user.name.matches_regex("^User")
/// ```
///
/// Typed options can be supplied with [`RegexOption`]:
///
/// ```rust,ignore
/// user.name.matches_regex_with_options(
///     "^user",
///     [RegexOption::CaseInsensitive],
/// )
/// ```
///
/// Literal prefix, suffix, and substring helpers are also available:
///
/// ```rust,ignore
/// user.name.starts_with("User")
/// user.name.ends_with("1")
/// user.name.contains_text("ser")
/// ```
///
/// These helpers escape regular-expression metacharacters before creating
/// the query.
///
/// # Numeric and bitwise queries
///
/// Numeric fields support MongoDB `$mod`:
///
/// ```rust,ignore
/// user.login_count.modulo(2, 0)
/// ```
///
/// Integer fields support all four MongoDB bitwise query operators:
///
/// ```rust,ignore
/// user.permissions.bits_all_set(0b0101)
/// user.permissions.bits_any_set(0b1100)
/// user.permissions.bits_all_clear(0b1100)
/// user.permissions.bits_any_clear(0b1100)
/// ```
///
/// # Array queries
///
/// Match an array containing one value:
///
/// ```rust,ignore
/// user.tags.contains("rust")
/// ```
///
/// Match an array containing every supplied value:
///
/// ```rust,ignore
/// user.tags.contains_all([
///     "rust",
///     "mongodb",
/// ])
/// ```
///
/// Match an exact array length:
///
/// ```rust,ignore
/// user.tags.has_size(2)
/// ```
///
/// Scalar arrays support typed `$elemMatch` conditions:
///
/// ```rust,ignore
/// user.scores.elem_match(|score| {
///     score.gte(60)
///         & score.lte(100)
/// })
/// ```
///
/// Arrays of embedded models support generated typed-field access:
///
/// ```rust,ignore
/// user.addresses.elem_match_nested(
///     |address| {
///         address.city.eq("City1")
///             & address.active.eq(true)
///     },
/// )
/// ```
///
/// # Embedded models
///
/// Derive [`Model`] with `#[model(embedded)]` for nested types and call
/// `.nested()` to access their generated typed fields:
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{
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
///     Serialize,
///     Deserialize,
///     Model,
/// )]
/// #[model(embedded)]
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
///         user.addresses.elem_match_nested(
///             |address| {
///                 address.city_name.eq("City1")
///                     & address.active.eq(true)
///             },
///         )
///     })
///     .all()
///     .await?;
///
/// # let _ = users;
/// # let _ = users_with_matching_address;
/// # Ok(())
/// # }
/// ```
///
/// Embedded models support required fields, `Option<T>`, `Vec<T>`, and
/// multiple nesting levels. Nested field paths respect Serde `rename` and
/// `rename_all` attributes.
///
/// # Sorting
///
/// Set a primary sort with `.sort_by()`:
///
/// ```rust,ignore
/// User::query()
///     .sort_by(|user| user.age.desc())
/// ```
///
/// Append secondary fields with `.then_sort_by()`:
///
/// ```rust,ignore
/// User::query()
///     .sort_by(|user| user.role.asc())
///     .then_sort_by(|user| user.age.desc())
///     .then_sort_by(|user| user.name.asc())
/// ```
///
/// Sort precedence follows insertion order.
///
/// # Limiting and pagination
///
/// Skip and limit matching results:
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
/// This calculates an offset of `10` and a limit of `10`.
///
/// Invalid page numbers, page sizes, and pagination overflow are returned as
/// [`QueryError`] when the query is executed.
///
/// # Text search
///
/// Collections with a text index can be searched with `.text()`:
///
/// ```rust,ignore
/// Article::query()
///     .text("rust mongodb")
///     .sort_by_text_score()
///     .all()
///     .await?
/// ```
///
/// Use [`TextSearch`] to configure language, case sensitivity, and diacritic
/// sensitivity:
///
/// ```rust,ignore
/// Article::query()
///     .text(
///         TextSearch::new("Café")
///             .language("none")
///             .case_sensitive(true)
///             .diacritic_sensitive(true),
///     )
///     .all()
///     .await?
/// ```
///
/// Text-search strings may also contain quoted phrases and excluded terms:
///
/// ```rust,ignore
/// Article::query()
///     .text("\"rust mongodb\" -beginner")
/// ```
///
/// # Geospatial queries
///
/// GeoJSON point fields with a `2dsphere` index support `$near`:
///
/// ```rust,ignore
/// Place::query()
///     .filter(|place| {
///         place.location.near(
///             GeoPoint::new(
///                 -79.38,
///                 43.65,
///             ),
///         )
///     })
///     .all()
///     .await?
/// ```
///
/// Use [`NearQuery`] for distance limits:
///
/// ```rust,ignore
/// place.location.near(
///     NearQuery::new(
///         GeoPoint::new(
///             -79.38,
///             43.65,
///         ),
///     )
///     .min_distance(500.0)
///     .max_distance(5_000.0),
/// )
/// ```
///
/// GeoJSON values also support `$geoWithin` and `$geoIntersects`:
///
/// ```rust,ignore
/// place.location.geo_within(boundary)
/// region.boundary.geo_intersects(point)
/// ```
///
/// Coordinates use longitude-latitude order. Distances for GeoJSON `$near`
/// queries are expressed in metres.
///
/// # Execution
///
/// Retrieve all matching models:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.active.eq(true))
///     .all()
///     .await?
/// ```
///
/// Retrieve the first matching model:
///
/// ```rust,ignore
/// User::query()
///     .sort_by(|user| user.created_at.asc())
///     .first()
///     .await?
/// ```
///
/// Count matching documents:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.active.eq(true))
///     .count()
///     .await?
/// ```
///
/// # Typed updates
///
/// Update and return the first matching document:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.name.eq("User1"))
///     .update_one(|user| {
///         user.active.set(true)
///             & user.login_count.inc(1)
///     })
///     .await?
/// ```
///
/// Update every matching document:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.active.eq(false))
///     .update_all(|user| {
///         user.status.set("inactive")
///     })
///     .await?
/// ```
///
/// Supported typed update helpers include:
///
/// ```rust,ignore
/// user.name.set("User1")
/// user.nickname.unset()
/// user.login_count.inc(1)
/// user.score.mul(2)
/// user.score.min(10)
/// user.score.max(100)
/// user.nickname.rename_to(
///     &user.display_name,
/// )
/// user.updated_at.current_date()
/// ```
///
/// Array update helpers include:
///
/// ```rust,ignore
/// user.tags.push("rust")
/// user.tags.push_each(["rust", "mongodb"])
/// user.tags.add_to_set("rust")
/// user.tags.add_each_to_set(["rust", "mongodb"])
/// user.tags.pull("deprecated")
/// user.tags.pop_first()
/// user.tags.pop_last()
/// ```
///
/// Array elements in embedded-document arrays can be updated through the
/// positional `$` and filtered positional `$[identifier]` operators.
///
/// Bulk updates reject sorting, skipping, limiting, and pagination.
///
/// # Typed deletions
///
/// Delete and return the first matching document:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.active.eq(false))
///     .sort_by(|user| user.created_at.asc())
///     .delete_one()
///     .await?
/// ```
///
/// Delete all matching documents:
///
/// ```rust,ignore
/// User::query()
///     .filter(|user| user.active.eq(false))
///     .delete_all()
///     .await?
/// ```
///
/// Bulk deletions reject sorting, skipping, limiting, and pagination.
///
/// An unfiltered `.update_all()` or `.delete_all()` operation affects every
/// document in the model's collection.
pub use oximod_core::query::Queryable;

/// An option that modifies MongoDB regular-expression matching.
///
/// Multiple options can be combined when calling
/// `matches_regex_with_options()`.
///
/// # Variants
///
/// - [`RegexOption::CaseInsensitive`] uses MongoDB option `"i"`.
/// - [`RegexOption::Multiline`] uses MongoDB option `"m"`.
/// - [`RegexOption::DotMatchesNewLine`] uses MongoDB option `"s"`.
/// - [`RegexOption::IgnoreWhitespace`] uses MongoDB option `"x"`.
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
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     Debug,
///     Serialize,
///     Deserialize,
///     Model,
/// )]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///
///     name: String,
/// }
///
/// # async fn run() -> Result<(), OxiModError> {
/// let users = User::query()
///     .filter(|user| {
///         user.name.matches_regex_with_options(
///             "^user",
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

/// A type-safe MongoDB update expression.
///
/// Update expressions are produced by methods on generated model fields and
/// returned from the closures passed to `.update_one()` and `.update_all()`.
///
/// Expressions can be combined with `&`. Updates using the same MongoDB
/// operator are merged into one operator document.
///
/// # Example
///
/// ```rust,ignore
/// let updated_user = User::query()
///     .filter(|user| {
///         user.name.eq("User1")
///     })
///     .update_one(|user| {
///         user.active.set(true)
///             & user.login_count.inc(1)
///             & user.nickname.unset()
///     })
///     .await?;
/// ```
///
/// When the same operator updates the same field more than once, the
/// expression on the right takes precedence.
pub use oximod_core::query::UpdateExpression;

/// A BSON type accepted by MongoDB's `$type` query operator.
///
/// Each variant maps to MongoDB's canonical string alias, such as `"string"`,
/// `"objectId"`, `"date"`, or `"long"`.
///
/// `$type` checks the BSON representation stored in MongoDB. It does not
/// deserialize or convert the value before matching.
///
/// # Example
///
/// ```rust,ignore
/// let users = User::query()
///     .filter(|user| {
///         user.nickname.has_bson_type(
///             BsonType::String,
///         )
///     })
///     .all()
///     .await?;
/// ```
pub use oximod_core::query::BsonType;

/// Configuration for a MongoDB `$text` search.
///
/// A string can be passed directly to `.text()` for a basic search. Use
/// `TextSearch` when language, case-sensitivity, or diacritic-sensitivity
/// options are required.
///
/// # Example
///
/// ```rust
/// use oximod::TextSearch;
///
/// let search = TextSearch::new(
///     "\"rust mongodb\" -beginner",
/// )
/// .language("none")
/// .case_sensitive(true)
/// .diacritic_sensitive(true);
/// ```
///
/// The collection must have an appropriate MongoDB text index before the query
/// can be executed.
pub use oximod_core::query::TextSearch;

/// A GeoJSON point.
///
/// Coordinates must be provided in longitude-latitude order.
///
/// # Example
///
/// ```rust
/// use oximod::GeoPoint;
///
/// let point = GeoPoint::new(
///     -79.38,
///     43.65,
/// );
///
/// assert_eq!(point.longitude(), -79.38);
/// assert_eq!(point.latitude(), 43.65);
/// ```
///
/// The serialized BSON representation is equivalent to:
///
/// ```text
/// {
///     "type": "Point",
///     "coordinates": [-79.38, 43.65]
/// }
/// ```
///
/// OxiMod does not validate coordinate ranges.
pub use oximod_core::query::GeoPoint;

/// A single-ring GeoJSON polygon.
///
/// [`GeoPolygon::new`] accepts the exterior ring in longitude-latitude order.
/// When the supplied ring is not already closed, its first coordinate is
/// appended automatically.
///
/// # Example
///
/// ```rust
/// use oximod::GeoPolygon;
///
/// let polygon = GeoPolygon::new([
///     [-1.0, -1.0],
///     [1.0, -1.0],
///     [1.0, 1.0],
///     [-1.0, 1.0],
/// ]);
/// ```
///
/// OxiMod closes the exterior ring but does not otherwise validate polygon
/// geometry.
///
/// `GeoPolygon::default()` exists for generated model-builder compatibility
/// and represents an empty polygon. Replace it before persistence.
pub use oximod_core::query::GeoPolygon;

/// Configuration for a MongoDB `$near` query.
///
/// Pass a [`GeoPoint`] directly to `.near()` for a basic proximity query. Use
/// `NearQuery` to configure minimum or maximum distance.
///
/// Distances are expressed in metres for GeoJSON queries using a MongoDB
/// `2dsphere` index.
///
/// # Example
///
/// ```rust
/// use oximod::{
///     GeoPoint,
///     NearQuery,
/// };
///
/// let query = NearQuery::new(
///     GeoPoint::new(
///         -79.38,
///         43.65,
///     ),
/// )
/// .min_distance(500.0)
/// .max_distance(5_000.0);
/// ```
///
/// OxiMod does not validate that distances are non-negative or that the
/// minimum does not exceed the maximum. MongoDB validates the resulting query.
pub use oximod_core::query::NearQuery;

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
        DateQueryValue, ElementExpression, ElementField, Expression, Field, FieldSchema,
        GeoGeometry, GeoPointQueryValue, GeoQueryValue, IntegerQueryValue, NumericQueryValue,
        OrderedQueryValue, Query, SortExpression, StringQueryValue,
    };
}
