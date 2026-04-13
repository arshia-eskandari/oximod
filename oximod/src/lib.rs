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
