//! # OxiMod
//!
//! Schema-aware MongoDB modeling for Rust.
//!
//! OxiMod is a modeling layer built on top of the official MongoDB Rust
//! driver. It adds derive-generated model construction, validation, defaults,
//! indexes, lifecycle hooks, persistence helpers, and typed queries while
//! preserving direct access to `mongodb::Collection` whenever the driver is the
//! better tool.
//!
//! OxiMod is therefore not a replacement for the MongoDB driver. It is a
//! model-oriented API that can be used alongside the driver without locking an
//! application into a reduced MongoDB feature set.
//!
//! ## Main capabilities
//!
//! - collection-backed and embedded models through one [`Model`] derive;
//! - fluent construction with generated setters and field defaults;
//! - aggregated field validation and custom validators;
//! - single-field MongoDB index declarations;
//! - optional save and `_id`-helper lifecycle hooks;
//! - global-client and explicit-client model operations;
//! - typed and raw MongoDB collection access;
//! - type-aware filters, sorting, pagination, text search, and geospatial
//!   queries;
//! - typed single-document and bulk updates and deletions.
//!
//! ## Quick start
//!
//! Collection-backed models require a database and collection name. Importing
//! [`Model`] brings both the derive macro and the persistence trait into scope.
//! Rust keeps macros and traits in separate namespaces, so the shared name is
//! intentional.
//!
//! ```rust,no_run
//! use mongodb::bson::{doc, oid::ObjectId};
//! use oximod::{Model, OxiClient, Queryable};
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
//!     let id = User::new()
//!         .email("alice@example.com")
//!         .name("Alice")
//!         .age(30)
//!         .active(true)
//!         .save()
//!         .await?;
//!
//!     let adults = User::query()
//!         .filter(|user| user.active.eq(true) & user.age.gte(18))
//!         .sort_by(|user| user.name.asc())
//!         .limit(20)
//!         .all()
//!         .await?;
//!
//!     User::update_by_id(id, doc! { "$set": { "active": true } }).await?;
//!
//!     println!("Found {} active adults", adults.len());
//!     Ok(())
//! }
//! ```
//!
//! ## Collection-backed and embedded models
//!
//! A collection-backed model owns a MongoDB collection and receives the public
//! [`Model`] persistence API plus [`Queryable`] typed queries:
//!
//! ```rust
//! use oximod::Model;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("app")]
//! #[collection("users")]
//! struct User {
//!     name: String,
//!     address: Address,
//! }
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Address {
//!     street: String,
//!     city: String,
//! }
//! ```
//!
//! An embedded model has no independent collection. It receives generated
//! construction, defaults, validation, and typed nested-field metadata, but it
//! does not implement [`Model`] or [`Queryable`] and cannot declare indexes or
//! lifecycle hooks.
//!
//! Embedded fields can still participate in collection-model queries:
//!
//! ```rust,no_run
//! # use oximod::{Model, Queryable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[db("app")]
//! # #[collection("users")]
//! # struct User {
//! #     name: String,
//! #     address: Address,
//! # }
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[model(embedded)]
//! # struct Address {
//! #     street: String,
//! #     city: String,
//! # }
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! User::query()
//!     .filter(|user| {
//!         user.address.nested(|address| {
//!             address.city.eq("City1")
//!         })
//!     })
//!     .all()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Generated nested paths honor supported Serde `rename` and `rename_all`
//! attributes.
//!
//! `#[serde(alias = "...")]` is a read-side compatibility tool, not a rename
//! migration. Typed query paths always use a field's primary serialized name,
//! so documents still stored under a legacy key remain readable through the
//! alias but are silently missed by typed filters on that field. During a
//! field rename, migrate the persisted documents — or match both spellings
//! with a raw `$or` filter through [`Model::get_document_collection`] —
//! before relying on typed queries against the renamed field.
//!
//! ## Construction and defaults
//!
//! Every derived model receives `new()`, [`Default::default`], and a fluent
//! setter for each field. Ordinary setters accept values through `Into<T>`;
//! setters for `Option<T>` accept a value convertible into `T` and store it as
//! `Some(...)`.
//!
//! `new()` is not a typestate builder. A field without `#[default(...)]` is
//! initialized with `Default::default()`. A configured default replaces that
//! fallback and may be any expression convertible into the field type:
//!
//! ```rust
//! use oximod::Model;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Preferences {
//!     #[default("Guest")]
//!     display_name: String,
//!
//!     #[default(false)]
//!     marketing_emails: bool,
//!
//!     nickname: Option<String>,
//! }
//!
//! let preferences = Preferences::new().nickname("Al");
//! assert_eq!(preferences.display_name, "Guest");
//! assert_eq!(preferences.nickname.as_deref(), Some("Al"));
//! ```
//!
//! `#[default(...)]` applies only while constructing a model in Rust; it is
//! never applied when reading documents from MongoDB. A stored document that
//! lacks the field entirely fails to deserialize regardless of any configured
//! `#[default(...)]`. When adding a field to a model whose collection already
//! contains documents, give the field a read-side default with
//! `#[serde(default = "path")]`. Avoid bare `#[serde(default)]` as a
//! schema-evolution strategy: it substitutes the field type's
//! `Default::default()` value — not the configured `#[default(...)]`
//! expression — and a later save writes that substituted value back to the
//! database.
//!
//! For a collection field named `_id`, the generated setter is `id()` by
//! default and accepts a MongoDB `ObjectId`. Rename it with
//! `#[document_id_setter_ident("with_id")]` when `id` would conflict with
//! another generated setter.
//!
//! ## Validation
//!
//! Both model kinds receive an inherent `validate()` method. Validation checks
//! every configured field and returns all failures together as
//! [`OxiModError::Validation`]. Calling [`Model::save`], [`Model::save_mut`],
//! [`Model::save_from`], or [`Model::save_from_mut`] validates immediately
//! before insertion, after the corresponding pre-save hook has run.
//!
//! ```rust
//! use oximod::{Model, OxiModError};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Profile {
//!     #[validate(min_length = 3, max_length = 32)]
//!     username: String,
//!
//!     #[validate(email)]
//!     email: String,
//! }
//!
//! let profile = Profile::new()
//!     .username("ab")
//!     .email("not-an-email");
//!
//! if let Err(error) = profile.validate() {
//!     for failure in error.validation_errors().unwrap_or_default() {
//!         println!("{}: {}", failure.field, failure.message);
//!     }
//! }
//! # let _: Option<OxiModError> = None;
//! ```
//!
//! Built-in validation groups include:
//!
//! - optional values: `required`;
//! - lengths: `min_length`, `max_length`, and `non_empty`;
//! - strings: `email`, `pattern`, `starts_with`, `ends_with`, `includes`, and
//!   `alphanumeric`;
//! - numbers: `min`, `max`, exclusive bounds, and sign rules;
//! - integers: `multiple_of`;
//! - user functions: `custom(path)`.
//!
//! The derive macro rejects incompatible built-in validators at compile time.
//! Custom validators return `Result<(), String>`. For `Option<T>`, validators
//! other than `required` operate on the inner value when it is present.
//!
//! Model validation is not automatically applied to raw update documents or
//! typed update expressions. Those operations modify stored documents directly
//! through MongoDB.
//!
//! ### Validation does not descend into embedded models
//!
//! Validating a model evaluates only the rules declared on that model's own
//! fields. `#[validate(...)]` rules declared inside an embedded model are
//! **not** evaluated when a containing model is validated or saved: the
//! parent can validate and save successfully while an embedded value violates
//! its own rules. This applies to every container shape, including bare
//! embedded fields, `Option<Embedded>`, and `Vec<Embedded>`. The embedded
//! type's own `validate()` method works normally when called directly.
//!
//! To enforce embedded rules through the parent, add a custom validator to
//! the containing field and delegate to the embedded value's `validate()`:
//!
//! ```rust
//! use oximod::Model;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Address {
//!     #[validate(non_empty)]
//!     city: String,
//! }
//!
//! fn validate_address(address: &Address) -> Result<(), String> {
//!     address.validate().map_err(|error| error.to_string())
//! }
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Profile {
//!     #[validate(custom(validate_address))]
//!     address: Address,
//! }
//!
//! let profile = Profile::new().address(Address::new().city(""));
//! assert!(profile.validate().is_err());
//! ```
//!
//! A pre-save hook can serve as an alternative save-time guard. When hooks
//! are used this way, implement **both** [`Hooks::pre_save`] and
//! [`Hooks::pre_save_mut`] if the application uses both save forms: `save()`
//! and `save_from()` run only `pre_save`, while `save_mut()` and
//! `save_from_mut()` run only `pre_save_mut`, so a guard implemented in one
//! hook does not protect the other save form.
//!
//! ## Indexes
//!
//! `#[index(...)]` declares a single-field index on a collection model. Common
//! options include scalar ordering, uniqueness, sparseness, TTL, text, hashed,
//! wildcard, hidden, collation, and `2d` or `2dsphere` geospatial settings.
//!
//! Deriving a model does not by itself create any server-side index.
//! Generated index initialization is attempted lazily before a model is
//! inserted. A successful initialization is remembered for that model type;
//! after a failed attempt, a later save can try again.
//!
//! Applications that need declared indexes before their first write can
//! establish them explicitly during startup with the generated
//! `ModelType::init_indexes()` method (global client) or
//! `ModelType::init_indexes_from(&client)` (explicit client). Both reuse the
//! save path's index machinery and share its once-per-process establishment
//! state: a successful explicit initialization is remembered exactly like a
//! save-triggered one, repeated successful calls are harmless, and a failed
//! index-establishment attempt returns the same error surface as
//! save-triggered establishment (an index error for server-side rejections,
//! a connection error for connectivity failures) and may be retried by a
//! later call or save. Applications that never call these
//! methods keep the existing lazy save-triggered behavior unchanged.
//!
//! Explicit initialization is establishment, not drift synchronization: it
//! does not verify server indexes on later calls, and an index dropped or
//! changed externally after a successful initialization is not automatically
//! re-established during the same process.
//!
//! Index creation is not triggered merely by constructing a query or obtaining
//! a collection. Use the MongoDB collection returned by [`Model::get_collection`]
//! for compound indexes or driver options not represented by `#[index(...)]`.
//!
//! Partial or filtered indexes (MongoDB's `partialFilterExpression` option)
//! are likewise not expressible with `#[index(...)]`; create them through the
//! driver's `create_index` on the collection returned by
//! [`Model::get_collection`] or [`Model::get_document_collection`].
//! Driver-created indexes coexist with `#[index(...)]` declarations. MongoDB
//! enforces that uniqueness; the underlying driver failure on a violation is
//! a duplicate-key error (E11000), not an OxiMod validation failure. OxiMod
//! validation does not replace MongoDB's index enforcement.
//!
//! Do not emulate a compound unique index by storing a derived composite-key
//! field guarded by `#[index(unique)]`. Partial updates such as
//! `update_by_id` or a typed `$set` can change the source fields without
//! recomputing the derived field, silently desynchronizing it; genuine
//! duplicates can then persist while the index still appears healthy. Create
//! a real MongoDB compound unique index through the driver instead.
//!
//! ## Clients and collection access
//!
//! [`OxiClient::init_global`] installs one process-wide MongoDB client. Model
//! methods without an `_from` suffix and all typed-query execution methods use
//! that global client. Treat `init_global` as a process-level, one-time
//! startup step and handle its `Result`: global-client operations require
//! initialization to have completed successfully — until it has, they fail —
//! and once a client is installed, later `init_global` calls return an error
//! rather than replacing it.
//!
//! Explicit-client model methods accept `&mongodb::Client`. An [`OxiClient`]
//! instance is one convenient way to own such a client:
//!
//! ```rust,no_run
//! use mongodb::bson::oid::ObjectId;
//! use oximod::{Model, OxiClient};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("app")]
//! #[collection("users")]
//! struct User {
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     _id: Option<ObjectId>,
//!     name: String,
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let owner = OxiClient::new("mongodb://localhost:27017".to_string()).await?;
//! let client = owner.client().expect("OxiClient::new initializes its client");
//!
//! let id = User::new().name("Alice").save_from(client).await?;
//! let user = User::find_by_id_from(id, client).await?;
//! # let _ = user;
//! # Ok(())
//! # }
//! ```
//!
//! OxiMod exposes both `mongodb::Collection<Self>` and
//! `mongodb::Collection<Document>` in global and explicit-client forms. These
//! escape hatches are intended for aggregation pipelines, driver options,
//! compound indexes, sessions, and any MongoDB operation outside OxiMod's
//! convenience APIs.
//!
//! Sessions and transactions in particular remain driver territory: no OxiMod
//! method accepts a `ClientSession`, so a model or typed-query write issued
//! while a transaction is open executes outside that transaction and commits
//! independently, with no error or warning. Once any write to a collection
//! participates in a transaction, perform every write to that collection
//! through the session-aware driver APIs on these collections for as long as
//! the transactional pattern is in use.
//!
//! ## Typed queries
//!
//! Import [`Queryable`] to call `ModelType::query()`. The closure-based API
//! receives a generated field structure whose methods depend on each field's
//! Rust type. Incompatible operations therefore fail to compile—for example,
//! regular expressions are unavailable on integers and array updates are
//! unavailable on scalar fields.
//!
//! Typed queries currently execute through the global [`OxiClient`]. There is
//! no explicit-client typed-query executor; use the `_from` model methods or an
//! explicitly obtained MongoDB collection when global state is unsuitable.
//!
//! The examples in this section use the following model:
//!
//! ```rust
//! use mongodb::bson::oid::ObjectId;
//! use oximod::Model;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Address {
//!     city: String,
//! }
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("app")]
//! #[collection("users")]
//! struct User {
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     _id: Option<ObjectId>,
//!     name: String,
//!     age: i32,
//!     active: bool,
//!     role: String,
//!     tags: Vec<String>,
//!     scores: Vec<i32>,
//!     addresses: Vec<Address>,
//!     login_count: i32,
//!     nickname: Option<String>,
//! }
//! ```
//!
//! ### Filters and logical expressions
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! # use oximod::{Model, Queryable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[model(embedded)]
//! # struct Address { city: String }
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[db("app")]
//! # #[collection("users")]
//! # struct User {
//! #     #[serde(skip_serializing_if = "Option::is_none")]
//! #     _id: Option<ObjectId>,
//! #     name: String,
//! #     age: i32,
//! #     active: bool,
//! #     role: String,
//! #     tags: Vec<String>,
//! #     scores: Vec<i32>,
//! #     addresses: Vec<Address>,
//! #     login_count: i32,
//! #     nickname: Option<String>,
//! # }
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! # let users =
//! User::query()
//!     .filter(|user| {
//!         user.active.eq(true)
//!             & user.age.gte(18)
//!             & (
//!                 user.role.eq("admin")
//!                     | user.role.eq("member")
//!             )
//!     })
//! #     .all()
//! #     .await?;
//! # let _ = users;
//! # Ok(())
//! # }
//! ```
//!
//! `&` creates logical AND and `|` creates logical OR; Rust does not permit
//! overloading `&&` and `||`. Repeated `filter()` calls are also combined with
//! AND. Field conditions can be negated with `not()`.
//!
//! Supported families include equality and membership, ordered comparisons,
//! existence and BSON-type checks, optional-field null checks, regular
//! expressions and escaped text helpers, numeric modulo, integer bitwise
//! predicates, arrays, embedded models, text search, and GeoJSON geospatial
//! predicates.
//!
//! Ordered comparisons, numeric updates, and modulo are also available on
//! `Option<T>` fields whose inner type supports them. The operand is always a
//! value of the inner type `T` — for example `nickname.lt("m")` on an
//! `Option<String>` field, or `inc(1)` on an `Option<i32>` field; `Some(...)`
//! and `None` operands do not compile. Documents storing BSON null and
//! documents missing the field follow MongoDB's normal query semantics: an
//! ordered comparison against an inner value does not match them.
//!
//! ### Arrays and embedded values
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! # use oximod::{Model, Queryable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[model(embedded)]
//! # struct Address { city: String }
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[db("app")]
//! # #[collection("users")]
//! # struct User {
//! #     #[serde(skip_serializing_if = "Option::is_none")]
//! #     _id: Option<ObjectId>,
//! #     name: String,
//! #     age: i32,
//! #     active: bool,
//! #     role: String,
//! #     tags: Vec<String>,
//! #     scores: Vec<i32>,
//! #     addresses: Vec<Address>,
//! #     login_count: i32,
//! #     nickname: Option<String>,
//! # }
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! # let users =
//! User::query()
//!     .filter(|user| {
//!         user.tags.contains_all(["rust", "mongodb"])
//!             & user.scores.elem_match(|score| {
//!                 score.gte(60) & score.lte(100)
//!             })
//!             & user.addresses.elem_match_nested(|address| {
//!                 address.city.eq("City1")
//!             })
//!     })
//! #     .all()
//! #     .await?;
//! # let _ = users;
//! # Ok(())
//! # }
//! ```
//!
//! Arrays support membership, `$all`, exact size, and scalar `$elemMatch`.
//! Arrays of embedded models add typed nested `$elemMatch` through
//! `elem_match_nested`, shown above, plus positional update helpers. An
//! `elem_match_nested` filter also supplies the array match that MongoDB's
//! positional `$` update operator requires, so pair it with `positional(...)`
//! when updating the first matched element.
//!
//! Array update operators such as `push`, `add_to_set`, `pull`, and
//! whole-array `set` require the element type to implement `Into<Bson>`.
//! Scalar elements qualify automatically; derived embedded models do not
//! implement `Into<Bson>` automatically. To use these operators on a
//! `Vec<Embedded>` field, implement the conversion once for the embedded
//! type:
//!
//! ```rust
//! use mongodb::bson::{Bson, to_bson};
//! use oximod::Model;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[model(embedded)]
//! struct Address {
//!     city: String,
//! }
//!
//! impl From<Address> for Bson {
//!     fn from(address: Address) -> Self {
//!         to_bson(&address).expect("Address serializes to BSON")
//!     }
//! }
//! ```
//!
//! Matching with `elem_match_nested` does not require this conversion; only
//! the mutation operators do. `From` conversions cannot report failure, so
//! the implementation must decide how to handle a value that fails to
//! serialize (this example panics via `expect`).
//!
//! ### Sorting, pagination, and execution
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! # use oximod::{Model, Queryable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[model(embedded)]
//! # struct Address { city: String }
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[db("app")]
//! # #[collection("users")]
//! # struct User {
//! #     #[serde(skip_serializing_if = "Option::is_none")]
//! #     _id: Option<ObjectId>,
//! #     name: String,
//! #     age: i32,
//! #     active: bool,
//! #     role: String,
//! #     tags: Vec<String>,
//! #     scores: Vec<i32>,
//! #     addresses: Vec<Address>,
//! #     login_count: i32,
//! #     nickname: Option<String>,
//! # }
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! let users = User::query()
//!     .filter(|user| user.active.eq(true))
//!     .sort_by(|user| user.role.asc())
//!     .then_sort_by(|user| user.name.asc())
//!     .page(2, 25)
//!     .all()
//!     .await?;
//! # let _ = users;
//! # Ok(())
//! # }
//! ```
//!
//! Pagination is one-based. `all()` applies sorting, skip, limit, and
//! pagination. `first()` applies sorting but not skip, limit, or pagination.
//! `count()` uses only the filter and text search. Invalid pagination and
//! out-of-range limits are returned as [`QueryError`] when the query executes.
//!
//! `all()` deserializes every document in the selected result window and
//! fails as a whole — returning `Err` and no documents — if any document in
//! that window cannot be deserialized into the model. Documents are never
//! silently skipped. With `page()`, each page is its own window: a page whose
//! window contains an undeserializable document errors, while later pages
//! whose windows contain only valid documents still succeed. Diagnose or
//! repair such documents through the raw collection returned by
//! [`Model::get_document_collection`].
//!
//! ### Text and geospatial queries
//!
//! Add `$text` with a string or [`TextSearch`] and optionally sort by MongoDB's
//! relevance score:
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! use oximod::{Model, Queryable, TextSearch};
//! # use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("app")]
//! #[collection("articles")]
//! struct Article {
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     _id: Option<ObjectId>,
//!
//!     #[index(text)]
//!     content: String,
//! }
//!
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! Article::query()
//!     .text(
//!         TextSearch::new("\"rust mongodb\" -beginner")
//!             .language("none"),
//!     )
//!     .sort_by_text_score()
//!     .all()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Text queries require an appropriate MongoDB text index.
//!
//! GeoJSON point and polygon fields support `$near`, `$geoWithin`, and
//! `$geoIntersects` where their typed field constraints permit:
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! use oximod::{GeoPoint, Model, NearQuery, Queryable};
//! # use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Model)]
//! #[db("app")]
//! #[collection("places")]
//! struct Place {
//!     #[serde(skip_serializing_if = "Option::is_none")]
//!     _id: Option<ObjectId>,
//!
//!     #[index(geo_2dsphere)]
//!     location: GeoPoint,
//! }
//!
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! Place::query()
//!     .filter(|place| {
//!         place.location.near(
//!             NearQuery::new(GeoPoint::new(-79.38, 43.65))
//!                 .max_distance(5_000.0),
//!         )
//!     })
//!     .all()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Coordinates use longitude-latitude order. Distances for GeoJSON `$near`
//! queries with a `2dsphere` index are expressed in metres.
//!
//! ### Typed writes
//!
//! `update_one()` and `delete_one()` apply filtering and sorting and return the
//! affected document. They do not apply skip, limit, or pagination.
//!
//! ```rust,no_run
//! # use mongodb::bson::oid::ObjectId;
//! # use oximod::{Model, Queryable};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[model(embedded)]
//! # struct Address { city: String }
//! # #[derive(Debug, Serialize, Deserialize, Model)]
//! # #[db("app")]
//! # #[collection("users")]
//! # struct User {
//! #     #[serde(skip_serializing_if = "Option::is_none")]
//! #     _id: Option<ObjectId>,
//! #     name: String,
//! #     age: i32,
//! #     active: bool,
//! #     role: String,
//! #     tags: Vec<String>,
//! #     scores: Vec<i32>,
//! #     addresses: Vec<Address>,
//! #     login_count: i32,
//! #     nickname: Option<String>,
//! # }
//! # async fn run() -> Result<(), oximod::OxiModError> {
//! let updated = User::query()
//!     .filter(|user| user.name.eq("User1"))
//!     .update_one(|user| {
//!         user.active.set(true)
//!             & user.login_count.inc(1)
//!             & user.nickname.unset()
//!     })
//!     .await?;
//! # let _ = updated;
//! # Ok(())
//! # }
//! ```
//!
//! `update_all()` and `delete_all()` operate on every matching document and
//! reject sorting, skipping, limiting, and pagination instead of silently
//! ignoring them. An unfiltered bulk write affects the entire collection.
//! Array filters configured with `array_filter()` are applied to typed update
//! operations.
//!
//! ## Lifecycle hooks
//!
//! Add `#[hooks]` to a collection model and implement [`Hooks`] to opt into
//! lifecycle calls. Hooks wrap only these global and explicit-client [`Model`]
//! helpers:
//!
//! - `save` and `save_from`;
//! - `save_mut` and `save_from_mut`;
//! - `find_by_id` and `find_by_id_from`;
//! - `update_by_id` and `update_by_id_from`;
//! - `delete_by_id` and `delete_by_id_from`.
//!
//! They do not wrap typed-query execution, direct collection operations,
//! `clear`, `exists`, or `count`. A pre-hook error prevents the database
//! operation. A post-hook error is returned after the database operation has
//! already succeeded.
//!
//! ## Errors
//!
//! [`OxiModError`] classifies failures by failure class: MongoDB driver
//! errors produced while executing OxiMod database operations are classified
//! by failure class rather than by the method that was executing, with a
//! fixed precedence: connectivity and client-infrastructure failures are
//! `Connection`; BSON encoding and decoding failures are `Serialization`, in
//! both directions; remaining index-domain and aggregation-domain failures
//! are `Index` and `Aggregation`; and every other driver failure — including
//! duplicate-key rejections — is `Database`, the conservative
//! non-connectivity fallback. MongoDB client construction and setup remain a
//! connection concern reported directly as `Connection`, outside the
//! operation-time classifier. The `GlobalClientInit`, `GlobalClientMissing`,
//! `Validation`, `Custom`, and `Query` variants keep their lifecycle,
//! validation, user-defined, and typed-query meanings and are not selected
//! by the operation-time driver classifier.
//!
//! `Connection` classifies the failure only: it does not guarantee that the
//! operation never reached MongoDB, and it does not make retrying safe —
//! retry safety depends on the specific operation, its idempotency, and
//! application policy. `Database` likewise does not guarantee that the
//! server definitely received or rejected the operation.
//!
//! Driver-backed variants retain the original `mongodb::error::Error` as
//! their [`source`](std::error::Error::source); downcast it to recover
//! server detail such as duplicate-key code 11000. `Display` text carries
//! human-readable operation context and is not a classification API.
//! Validation and query details can be inspected through
//! [`OxiModError::validation_errors`] and [`OxiModError::query_error`].
//!
//! Variant matchers written against OxiMod 0.3.0 may observe different arms:
//! 0.3.0 selected variants by call site. In particular, a duplicate key
//! through `save()` is now `Database` rather than `Connection`, an
//! unreachable server is `Connection` through every operation, and an
//! undeserializable document is `Serialization` through every read path.
//!
//! For complete runnable examples, see the
//! [`examples/`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples)
//! directory.

// --- Public errors ---------------------------------------------------------

/// Primary error returned by OxiMod operations.
///
/// The variants identify failure classes — connectivity, global-client
/// lifecycle, BSON encoding/decoding, aggregation-domain, index-domain,
/// validation, general database operation, user-defined, and typed-query
/// configuration failures. MongoDB driver errors produced while executing
/// database operations are classified by failure class rather than by the
/// method that was executing; client construction and setup failures are
/// reported directly as connection failures.
///
/// Errors backed by another library retain that error as their source. Use
/// [`OxiModError::validation_errors`] or [`OxiModError::query_error`] to inspect
/// structured validation and query failures without pattern matching.
pub use oximod_core::error::oximod_error::OxiModError;

/// Invalid typed-query configuration detected during construction or execution.
///
/// This includes zero page numbers or sizes, pagination overflow, limits that
/// cannot be represented by the MongoDB driver, and unsupported modifiers on
/// bulk writes. It is normally returned through [`OxiModError::Query`].
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::oid::ObjectId;
/// use oximod::{Model, OxiModError, QueryError, Queryable};
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
/// match User::query().page(0, 10).all().await {
///     Err(OxiModError::Query(QueryError::InvalidPageNumber { page })) => {
///         println!("Invalid page number: {page}");
///     }
///     Err(error) => return Err(error),
///     Ok(users) => println!("Found {} users", users.len()),
/// }
/// # Ok(())
/// # }
/// ```
pub use oximod_core::error::query_error::QueryError;

/// A typed bulk-write operation named by [`QueryError`].
pub use oximod_core::error::query_error::BulkWriteOperation;

/// A query modifier rejected by a typed bulk write.
pub use oximod_core::error::query_error::QueryModifier;

/// One field-level validation failure.
///
/// The public [`ValidationError::field`] and [`ValidationError::message`]
/// members identify the failed model field and describe the violated rule.
pub use oximod_core::error::oximod_error::ValidationError;

/// All field-level failures collected during one model validation pass.
///
/// The tuple field contains a `Vec<ValidationError>` and is also exposed as a
/// slice through [`OxiModError::validation_errors`].
pub use oximod_core::error::oximod_error::ValidationErrors;

// --- Clients and model runtime --------------------------------------------

/// MongoDB client wrapper used by OxiMod.
///
/// `OxiClient` supports two independent patterns:
///
/// - [`OxiClient::init_global`] and [`OxiClient::global`] manage the single
///   process-wide client used by ordinary model methods and typed queries;
/// - [`OxiClient::new`] or [`OxiClient::init_client`] owns an instance-level
///   client retrievable through [`OxiClient::client`] or
///   [`OxiClient::client_mut`].
///
/// Instance-level clients are useful for tests, dependency injection, and
/// applications that communicate with more than one MongoDB deployment. Model
/// `_from` methods take `&mongodb::Client`, not `&OxiClient`.
///
/// # Global client
///
/// ```rust,no_run
/// use oximod::OxiClient;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// OxiClient::init_global("mongodb://localhost:27017".to_string()).await?;
/// let client = OxiClient::global()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
///
/// # Instance-level client
///
/// ```rust,no_run
/// use oximod::OxiClient;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let owner = OxiClient::new("mongodb://localhost:27017".to_string()).await?;
/// let client = owner.client().expect("new clients are initialized");
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
pub use oximod_core::feature::conn::client::OxiClient;

/// Lifecycle hooks for collection-backed models.
///
/// Hooks are enabled with `#[hooks]` and have default no-op implementations.
/// Override only the methods a model needs.
///
/// The hook surface covers immutable and mutable saves plus `_id`-based find,
/// update, and delete helpers in both global-client and explicit-client forms.
/// Hooks do not wrap typed queries or direct MongoDB collection operations.
///
/// Each save form runs only its own hooks: `save()` and `save_from()` run
/// `pre_save`/`post_save`, while `save_mut()` and `save_from_mut()` run
/// `pre_save_mut`/`post_save_mut`. A safeguard implemented in only one
/// pre-save hook does not guard the other save form; implement both when the
/// application uses both.
///
/// A pre-hook error aborts the database operation. A post-hook error reports a
/// post-processing failure after the database operation has succeeded.
///
/// # Example
///
/// ```rust,no_run
/// # use mongodb::bson::oid::ObjectId;
/// # use oximod::{Hooks, Model, OxiModError};
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Debug, Serialize, Deserialize, Model)]
/// # #[db("app")]
/// # #[collection("users")]
/// # #[hooks]
/// # struct User {
/// #     #[serde(skip_serializing_if = "Option::is_none")]
/// #     _id: Option<ObjectId>,
/// #     email: String,
/// # }
/// #[async_trait::async_trait]
/// impl Hooks for User {
///     async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
///         self.email = self.email.trim().to_lowercase();
///         Ok(())
///     }
/// }
/// ```
pub use oximod_core::feature::hooks::Hooks;

/// Persistence interface implemented by collection-backed OxiMod models.
///
/// Importing `oximod::Model` brings this trait and the derive macro of the same
/// name into scope.
///
/// The trait provides:
///
/// - typed and raw collection access;
/// - immutable and mutable saves;
/// - find, update, and delete helpers by MongoDB `ObjectId`;
/// - collection clearing;
/// - existence checks and document counts;
/// - explicit-client counterparts for every operation.
///
/// Methods without `_from` use the global [`OxiClient`]. Methods ending in
/// `_from` accept `&mongodb::Client`. Embedded models do not implement this
/// trait.
///
/// # Example
///
/// ```rust,no_run
/// use mongodb::bson::{doc, oid::ObjectId};
/// use oximod::{Model, OxiModError};
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
/// let id = User::new().name("Alice").save().await?;
/// let found = User::find_by_id(id).await?;
/// let exists = User::exists(doc! { "name": "Alice" }).await?;
/// # let _ = (found, exists);
/// # Ok(())
/// # }
/// ```
pub use oximod_core::feature::model::Model;

/// Derive macro for collection-backed and embedded OxiMod models.
///
/// The derive supports named-field structs. All derived fields are initialized
/// by `new()`, so each field must either implement [`Default`] or define a
/// compatible `#[default(...)]` expression.
///
/// # Collection-backed models
///
/// Collection models are the default and require:
///
/// - `#[db("database_name")]`;
/// - `#[collection("collection_name")]`.
///
/// They receive construction, defaults, validation, index initialization,
/// optional hooks, [`Model`] persistence, and [`Queryable`] typed queries.
/// Optional struct-level attributes are:
///
/// - `#[hooks]`;
/// - `#[document_id_setter_ident("with_id")]`;
/// - `#[index_max_retries(N)]`;
/// - `#[index_max_init_seconds(N)]`.
///
/// The two index-initialization settings are accepted by the current derive and
/// stored in its coordinator. The current `run_once` implementation does not
/// enforce them as hard retry or timeout limits.
///
/// # Embedded models
///
/// `#[model(embedded)]` creates a value intended to be stored within another
/// model. Embedded models receive construction, defaults, validation, and
/// typed nested-field metadata. They do not receive persistence, root queries,
/// indexes, or hooks.
///
/// Collection-only attributes are rejected on embedded models:
///
/// - `db` and `collection`;
/// - `hooks` and `index`;
/// - `document_id_setter_ident`;
/// - `index_max_retries` and `index_max_init_seconds`.
///
/// # Field attributes
///
/// - `#[default(expression)]` replaces `Default::default()` during `new()`;
/// - `#[validate(...)]` adds compile-time-checked built-in or custom rules.
///   Rules apply to the model's own fields only: validation does not descend
///   into embedded models. See the crate-level Validation section for the
///   custom-validator and hook remedies;
/// - `#[index(...)]` declares a single-field MongoDB index on a collection
///   model. Declared indexes are not created by deriving the model: they are
///   established lazily before document insertion during save, or explicitly
///   at startup through the generated `init_indexes()` /
///   `init_indexes_from(&client)` methods. See the crate-level Indexes
///   section for the complete lifecycle.
///
/// Serde field and container renames are reflected in generated typed paths.
///
/// # Example
///
/// ```rust
/// use oximod::Model;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[model(embedded)]
/// #[serde(rename_all = "camelCase")]
/// struct Address {
///     street_name: String,
///     city: String,
/// }
///
/// #[derive(Debug, Serialize, Deserialize, Model)]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     #[validate(min_length = 2)]
///     name: String,
///
///     address: Address,
/// }
///
/// let user = User::new()
///     .name("User1")
///     .address(Address::new().street_name("Main St").city("City1"));
/// # let _ = user;
/// ```
pub use oximod_macros::Model;

// --- Typed queries ---------------------------------------------------------

/// Trait implemented by collection models that support typed queries.
///
/// `#[derive(Model)]` implements this trait only for collection-backed models.
/// Importing it brings [`Queryable::query`] into scope. The resulting builder
/// uses the global [`OxiClient`] when an execution method is called.
///
/// The query closure receives generated fields whose available methods are
/// determined by their Rust types and whose paths follow supported Serde
/// renames. Queries can express filtering, sorting, pagination, text and
/// geospatial search, typed updates, and typed deletion.
///
/// # Modifier behavior
///
/// - repeated filters are combined with logical AND;
/// - `sort_by` replaces the primary sort and `then_sort_by` appends;
/// - `all` applies sort, skip, limit, and one-based pagination;
/// - `first` applies sorting but ignores skip, limit, and pagination;
/// - `count` ignores sort, skip, limit, and pagination;
/// - `update_one` and `delete_one` apply sorting but not skip, limit, or
///   pagination;
/// - `update_all` and `delete_all` reject sort, skip, limit, and pagination.
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
///     age: i32,
///     active: bool,
/// }
///
/// # async fn run() -> Result<(), OxiModError> {
/// let users = User::query()
///     .filter(|user| user.active.eq(true) & user.age.gte(18))
///     .sort_by(|user| user.name.asc())
///     .limit(20)
///     .all()
///     .await?;
/// # let _ = users;
/// # Ok(())
/// # }
/// ```
///
/// # Bulk-write warning
///
/// An unfiltered `update_all()` or `delete_all()` affects every document in the
/// collection.
pub use oximod_core::query::Queryable;

/// Option accepted by typed MongoDB regular-expression queries.
///
/// The variants map to MongoDB's `i`, `m`, `s`, and `x` options:
/// case-insensitive, multiline, dot-matches-newline, and ignore-whitespace.
/// Multiple options can be supplied to `matches_regex_with_options()`.
pub use oximod_core::query::RegexOption;

/// Type-safe MongoDB update document assembled from generated fields.
///
/// Update expressions are returned by field helpers such as `set`, `unset`,
/// `inc`, `push`, and positional array updates. Combine independent operations
/// with `&`. Documents using the same MongoDB operator are merged; when the same
/// operator writes the same path more than once, the right-hand expression
/// takes precedence.
///
/// Update expressions write specific dotted field paths. When documents
/// written by older model versions may still be present, prefer these
/// targeted updates — or `$set` on dotted paths through
/// [`Model::update_by_id`] — over replacing a whole stored document with a
/// serialized model: a whole-document replacement writes only the current
/// struct shape and can drop or rewrite fields the running code no longer
/// declares.
///
/// # Example
///
/// ```rust,no_run
/// # use mongodb::bson::oid::ObjectId;
/// # use oximod::{Model, Queryable};
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Debug, Serialize, Deserialize, Model)]
/// # #[db("app")]
/// # #[collection("users")]
/// # struct User {
/// #     #[serde(skip_serializing_if = "Option::is_none")]
/// #     _id: Option<ObjectId>,
/// #     name: String,
/// #     active: bool,
/// #     login_count: i32,
/// #     nickname: Option<String>,
/// # }
/// # async fn run() -> Result<(), oximod::OxiModError> {
/// # let _ =
/// User::query()
///     .filter(|user| user.name.eq("User1"))
///     .update_one(|user| {
///         user.active.set(true)
///             & user.login_count.inc(1)
///             & user.nickname.unset()
///     })
///     .await?;
/// # Ok(())
/// # }
/// ```
pub use oximod_core::query::UpdateExpression;

/// BSON type accepted by a generated field's `$type` predicate.
///
/// Each variant maps to a MongoDB canonical string alias. The check is applied
/// to the BSON value stored in MongoDB; no Rust deserialization or conversion
/// happens before matching.
pub use oximod_core::query::BsonType;

/// Configuration for a MongoDB `$text` query.
///
/// Pass a string directly to `query().text(...)` for a basic search. Use this
/// type to configure language, case sensitivity, or diacritic sensitivity.
/// MongoDB phrase and excluded-term syntax can be included in the search text.
///
/// # Example
///
/// ```rust
/// use oximod::TextSearch;
///
/// let search = TextSearch::new("\"rust mongodb\" -beginner")
///     .language("none")
///     .case_sensitive(true)
///     .diacritic_sensitive(true);
/// # let _ = search;
/// ```
///
/// The target collection must have an appropriate text index.
pub use oximod_core::query::TextSearch;

/// GeoJSON point stored as longitude and latitude.
///
/// Its serialized representation is a GeoJSON object with type `"Point"` and
/// coordinates `[longitude, latitude]`. OxiMod does not validate coordinate
/// ranges.
///
/// # Example
///
/// ```rust
/// use oximod::GeoPoint;
///
/// let point = GeoPoint::new(-79.38, 43.65);
/// assert_eq!(point.longitude(), -79.38);
/// assert_eq!(point.latitude(), 43.65);
/// ```
pub use oximod_core::query::GeoPoint;

/// Single-ring GeoJSON polygon.
///
/// [`GeoPolygon::new`] accepts an exterior ring in longitude-latitude order and
/// appends its first coordinate when the ring is not closed. OxiMod does not
/// otherwise validate the polygon's geometry.
///
/// `GeoPolygon::default()` is an empty polygon provided so generated model
/// construction remains possible. Replace it before persistence.
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
/// # let _ = polygon;
/// ```
pub use oximod_core::query::GeoPolygon;

/// Configuration for a typed MongoDB `$near` query.
///
/// A [`GeoPoint`] can be passed directly to `near()` for a basic query. This
/// type adds optional minimum and maximum distances. With GeoJSON and a
/// `2dsphere` index, distances are expressed in metres.
///
/// OxiMod does not validate that distances are non-negative or that the minimum
/// does not exceed the maximum; MongoDB validates the resulting query.
///
/// # Example
///
/// ```rust
/// use oximod::{GeoPoint, NearQuery};
///
/// let query = NearQuery::new(GeoPoint::new(-79.38, 43.65))
///     .min_distance(500.0)
///     .max_distance(5_000.0);
/// # let _ = query;
/// ```
pub use oximod_core::query::NearQuery;

// --- Generated-code support ----------------------------------------------

#[doc(hidden)]
pub use async_trait as _async_trait;

#[doc(hidden)]
pub mod _error {
    //! Hidden macro-support namespace for generated error handling.
    //!
    //! Generated code routes operation-time MongoDB driver errors through
    //! the centralized classifier here. This namespace is not supported
    //! public API; classify failures through the public [`OxiModError`]
    //! variants and `source()` instead.
    //!
    //! [`OxiModError`]: crate::OxiModError

    pub use oximod_core::error::classify::{OperationDomain, classify_driver_error};
}

#[doc(hidden)]
pub use futures_util as _futures_util;

#[doc(hidden)]
pub use mongodb as _mongodb;

#[doc(hidden)]
pub use oximod_core::feature as _feature;

#[doc(hidden)]
pub use oximod_core::helpers as _helpers;

#[doc(hidden)]
pub use regex as _regex;

#[doc(hidden)]
pub mod _query {
    pub use oximod_core::query::{
        DateQueryValue, ElementExpression, ElementField, Expression, Field, FieldSchema,
        GeoGeometry, GeoPointQueryValue, GeoQueryValue, IntegerQueryValue, NumericQueryValue,
        OrderedQueryValue, Query, SortExpression, StringQueryValue,
    };
}
