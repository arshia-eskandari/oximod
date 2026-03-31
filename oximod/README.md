# OxiMod

<p align="center">
  <strong>Schema-aware MongoDB modeling for Rust</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/crates/v/oximod">
  <img src="https://img.shields.io/crates/d/oximod">
  <img src="https://img.shields.io/badge/license-MIT-blue">
</p>

---

## Overview

OxiMod is a schema-based modeling layer for MongoDB, designed for Rust developers who want a more expressive way to define models without giving up direct access to the MongoDB driver.

Inspired by ODM-style workflows, OxiMod provides:

- derive-based schema configuration  
- builder-style model construction  
- field-level validations, defaults, and indexes
- convenient model helpers  
- global and explicit-client workflows  
- optional lifecycle hooks  

At the same time, it preserves MongoDB’s native power by exposing:

- `mongodb::Collection<Self>`
- `mongodb::Collection<Document>`

OxiMod is best understood as:

> **MongoDB with stronger model ergonomics**, not a replacement for the driver.

This ensures:
- zero feature lock-in  
- full MongoDB flexibility  
- long-term maintainability  
---

## Example

```rust
use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Hooks, Model, OxiClient, OxiModError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
#[document_id_setter_ident("id_setter")]
#[hooks]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "email_idx", case_insensitive)]
    #[validate(email)]
    email: String,

    #[validate(min_length = 3, max_length = 32)]
    name: String,

    #[validate(non_negative, min = 18)]
    age: i32,

    #[default(false)]
    active: bool,
}

#[async_trait::async_trait]
impl Hooks for User {
    async fn pre_save_mut(&mut self) -> Result<(), OxiModError> {
        self.email = self.email.trim().to_lowercase();
        self.name = self.name.trim().to_string();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(uri).await?;

    User::clear().await?;

    // Builder API with Into<T> support and defaults
    let mut user = User::new()
        .email("  ALICE@EXAMPLE.COM  ")
        .name("Alice")
        .age(30);

    // save_mut() runs mutable hooks before persistence
    let id = user.save_mut().await?;
    println!("Inserted user: {}", id);

    let found = User::find_by_id(id).await?;
    println!("Found user: {found:#?}");

    User::update_by_id(id, doc! { "$set": { "active": true } }).await?;

    let active_exists = User::exists(doc! { "active": true }).await?;
    let total = User::count(doc! {}).await?;
    println!("Any active users? {}", active_exists);
    println!("Total users: {}", total);

    // Access the typed MongoDB collection directly when needed
    let collection = User::get_collection()?;
    let active_users = collection.count_documents(doc! { "active": true }).await?;
    println!("Active users counted via driver: {}", active_users);

    Ok(())
}
```

For more examples, feel free to check out the [`examples/`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples) directory.

---

## Model API

The following methods are automatically generated when deriving the `Model` macro and provide a typed interface for interacting with your MongoDB collection.

### Core

| Method/Associated Function | Signature | Description |
|------|-----------|------------|
| `save()` | `async fn save(&self) -> Result<ObjectId, OxiModError>` | Inserts the current model instance into the database. Runs validation and non-mutable hooks, and returns the inserted document’s `_id`. |
| `save_mut()` | `async fn save_mut(&mut self) -> Result<ObjectId, OxiModError>` | Inserts the model while allowing mutable hooks to modify it before persistence. Returns the inserted document’s `_id`. |
| `clear()` | `async fn clear() -> Result<DeleteResult, OxiModError>` | Deletes all documents in the model’s collection. Returns a `DeleteResult` indicating how many documents were removed. |
| `get_collection()` | `fn get_collection() -> Result<Collection<Self>, OxiModError>` | Returns the typed `mongodb::Collection<Self>` for performing advanced or custom queries. |
| `get_document_collection()` | `fn get_document_collection() -> Result<Collection<Document>, OxiModError>` | Returns the raw `mongodb::Collection<Document>` for working directly with BSON when full flexibility is needed. |


### Identity Helpers

| Associated Function | Signature | Description |
|------|-----------|------------|
| `find_by_id()` | `async fn find_by_id(id: ObjectId) -> Result<Option<Self>, OxiModError>` | Fetches a document by its `_id`. Returns `Some(model)` if found, otherwise `None`. |
| `update_by_id()` | `async fn update_by_id(id: ObjectId, update: Document) -> Result<UpdateResult, OxiModError>` | Updates a document by `_id` using a MongoDB update document (e.g., `$set`). Returns an `UpdateResult` describing the operation. |
| `delete_by_id()` | `async fn delete_by_id(id: ObjectId) -> Result<DeleteResult, OxiModError>` | Deletes a document by `_id`. Returns a `DeleteResult` indicating whether a document was removed. |


### Utilities

| Associated Function | Signature | Description |
|------|-----------|------------|
| `exists()` | `async fn exists(filter: Document) -> Result<bool, OxiModError>` | Checks if at least one document matches the given filter. Returns `true` if a match exists. |
| `count()` | `async fn count(filter: Document) -> Result<u64, OxiModError>` | Counts the number of documents matching the given filter and returns the total. |

---

## Builder API

OxiMod generates builder-style setter methods for models, making it easier to construct documents in a fluent and readable way.

By default, the generated `_id` setter is named `id()`. If you want a different name, you can customize it with the `#[document_id_setter_ident("id_setter")]` attribute.

```rust
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
#[document_id_setter_ident("id_setter")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,
    name: String,
    age: i32,
    active: bool,
}
```

Once derived, you can build a model instance like this:

```rust
let user = User::new()
    .id_settter(ObjectId::new())
    .name("Alice")
    .age(30)
    .active(true);
```

The builder API provides a fluent and ergonomic way to construct model instances with type-safe setters generated at compile time.

### Features

- **Flexible input types (`Into<T>`)**  
  Setter methods accept any type that can be converted into the field type, such as `&str` for a `String` field, which helps reduce unnecessary boilerplate.

- **Automatic type conversion**  
  Values are converted into their target field types internally, allowing concise and expressive model construction.

- **Default value support**  
  Fields annotated with `#[default(...)]` are automatically populated if they are not explicitly set by the builder.

- **Optional and required fields**  
  The builder supports both `Option<T>` and required fields, making it suitable for a wide range of model definitions.

- **Customizable `_id` setter**  
  By default, the generated setter for `_id` is `id()`. You can rename it with `#[document_id_setter_ident("id_setter")]` when a custom method name better fits your API.

---

## Client Usage

OxiMod supports two client access patterns: a **global client** for simple application-wide usage, and an **explicit client** for cases where you want more control over how connections are managed.

The global client is ideal when your application uses a single MongoDB connection shared across model operations. Explicit clients are better when you want to avoid global state or pass a client directly into your models and queries.

### Global

Use the global client when your application has a single primary MongoDB connection and you want OxiMod methods like `save()`, `find_by_id()`, and `count()` to work without manually passing a client every time.

Initialize it once, typically during application startup:

```rust
OxiClient::init_global(uri).await?;
user.save().await?;
```

After the global client has been initialized, model methods that do not take a client explicitly will automatically use it internally.

### Explicit

Use the explicit-client API when you want to pass a specific MongoDB client into each operation instead of relying on global state.

```rust
user.save_from(&client).await?;
```

This pattern is useful when you need tighter control over connection lifetimes or when different parts of your application may use different database clients.

Used for:
- tests
- multi-tenant applications
- dependency injection
- scoped or non-global connection management

### OxiClient API

| Method/Associated Function | Signature | Description |
|------|-----------|------------|
| `new()` | `async fn new(url: String) -> Result<OxiClient, OxiModError>` | Creates a new `OxiClient` instance by connecting to MongoDB using the provided connection string. The resulting client stores a local `mongodb::Client` internally. |
| `init_client()` | `async fn init_client(&mut self, mongo_uri: String) -> Result<(), OxiModError>` | Initializes or replaces the local MongoDB client stored inside an existing `OxiClient` instance. |
| `client()` | `fn client(&self) -> Option<&Client>` | Returns a reference to the local inner `mongodb::Client` if one has been initialized. |
| `client_mut()` | `fn client_mut(&mut self) -> Option<&Client>` | Returns access to the local inner `mongodb::Client` if initialized, for advanced driver-level usage. |
| `init_global()` | `async fn init_global(mongo_uri: String) -> Result<(), OxiModError>` | Initializes the global MongoDB client used by OxiMod APIs that do not receive an explicit client. This should typically be called once at application startup. |
| `global()` | `fn global() -> Result<Arc<Client>, OxiModError>` | Returns the globally initialized MongoDB client. Fails if `init_global()` has not been called yet. |

### Choosing between global and explicit clients

| Pattern | Best for |
|------|----------|
| Global client | Applications with a single shared MongoDB connection and a simpler setup. |
| Explicit client | Tests, multi-tenant systems, dependency injection, and cases where you want to avoid relying on global state. |

---

## Collections

OxiMod provides two ways to access MongoDB collections: **typed collections** for type-safe operations and **raw collections** for full flexibility.

### Typed

```rust
let collection = User::get_collection()?;
```

Returns `mongodb::Collection<Self>`, enabling type-safe queries and automatic (de)serialization. This is the recommended approach for most use cases.

#### API

| Method/Associated Function | Signature | Description |
|------|-----------|------------|
| `get_collection()` | `fn get_collection() -> Result<Collection<Self>, OxiModError>` | Gets the typed collection using the global client. |
| `get_collection_from()` | `fn get_collection_from(client: &Client) -> Result<Collection<Self>, OxiModError>` | Gets the typed collection using an explicit client. |

### Raw

```rust
let collection = User::get_document_collection()?;
```

Returns `mongodb::Collection<Document>`, allowing direct interaction with BSON documents.

#### API

| Associated Function | Signature | Description |
|------|-----------|------------|
| `get_document_collection()` | `fn get_document_collection() -> Result<Collection<Document>, OxiModError>` | Gets the raw collection using the global client. |
| `get_document_collection_from()` | `fn get_document_collection_from(client: &Client) -> Result<Collection<Document>, OxiModError>` | Gets the raw collection using an explicit client. |

### When to use each

| Type | Best for |
|------|----------|
| Typed | Everyday usage with type safety |
| Raw | Advanced queries and dynamic data |

---

## Attributes

OxiMod uses attributes to configure how your models behave at compile time.  
Struct-level attributes define how your model maps to MongoDB and how certain features (like indexes and hooks) behave.

### Struct-level

| Attribute | Description |
|----------|------------|
| `#[db("name")]` | **Required**. Specifies the MongoDB database name where this model will be stored. This is required for resolving the correct database at runtime. |
| `#[collection("name")]` | **Required**. Defines the MongoDB collection name associated with the model. All operations (save, query, delete) will target this collection. |
| `#[document_id_setter_ident("id_setter")]` | Renames the generated builder setter for the `_id` field. By default, the setter is `id()`, but this allows you to customize it (e.g., `id_setter()`). |
| `#[index_max_retries(N)]` | Sets how many times OxiMod will retry index creation during initialization. Useful for handling transient database issues during startup. |
| `#[index_max_init_seconds(N)]` | Specifies the maximum time (in seconds) allowed for index initialization before timing out. Helps prevent long startup delays. |
| `#[hooks]` | Enables lifecycle hooks (e.g., pre/post save). Without this attribute, hook-related logic is not generated, avoiding unnecessary overhead. |

#### Example

```rust
#[derive(Model)]
#[db("my_app_db")]
#[collection("users")]
#[document_id_setter_ident("id_setter")]
#[index_max_retries(3)]
#[index_max_init_seconds(10)]
#[hooks]
struct User {
    _id: Option<ObjectId>,
    name: String,
}
```

### Field-level

#### Indexing

Use `#[index(...)]` on a field to declare a MongoDB index directly in your model.  
These options let you define common index behavior close to the field itself, without dropping down to the full driver API.

```rust
#[index(...)]
```

##### Core

| Attribute | Description |
|----------|------------|
| `unique` | Creates a unique index, preventing duplicate values for the indexed field. |
| `sparse` | Excludes documents where the field is missing from the index. |
| `hidden` | Creates the index but hides it from the query planner unless explicitly hinted. |
| `name = "..."` | Assigns a custom name to the index instead of using MongoDB’s generated name. |
| `order = 1/-1` | Sets ascending (`1`) or descending (`-1`) order for a standard scalar index. |
| `expire_after_secs = N` | Creates a TTL index so documents expire automatically after `N` seconds. |
| `background` | Builds the index in the background to reduce disruption to database operations. |

##### Advanced Types

| Attribute | Description |
|----------|------------|
| `text` | Creates a text index for MongoDB text search. |
| `hashed` | Creates a hashed index, useful for hashed lookups and some sharding strategies. |
| `wildcard` | Creates a wildcard-style index for dynamic or document-like fields. |
| `geo_2dsphere` | Creates a 2dsphere geospatial index for spherical location queries. |
| `geo_2d` | Creates a 2d geospatial index for planar coordinate queries. |

##### Advanced Options

| Attribute | Description |
|----------|------------|
| `version = N` | Sets the version of a standard index structure when supported by MongoDB. |
| `text_index_version = N` | Sets the version of a text index. Only meaningful with `text`. |
| `geo_2dsphere_index_version = N` | Sets the version of a 2dsphere index. Only meaningful with `geo_2dsphere`. |
| `weight = N` | Assigns a weight to a text-indexed field to influence text search scoring. |
| `default_language = "..."` | Sets the default language used by a text index. |
| `language_override = "..."` | Specifies the document field that overrides the default text index language. |
| `case_insensitive` | Applies a case-insensitive collation preset for string-based lookups. |
| `bits = N` | Sets precision for a `geo_2d` index. |
| `min = N` | Sets the lower bound for a `geo_2d` index. |
| `max = N` | Sets the upper bound for a `geo_2d` index. |

##### Example

```rust
#[index(unique, sparse, name = "email_idx")]
email: String,

#[index(text, weight = 10, default_language = "english")]
title: String,

#[index(geo_2dsphere)]
location: GeoJsonPoint,
```

##### Notes

- These options are meant to cover common field-level index needs in a compact way.
- Specialized index types such as `text`, `hashed`, `wildcard`, `geo_2dsphere`, and `geo_2d` are generally used one at a time per field.
- Some options only make sense with specific index types. For example, `weight` applies to `text`, while `bits`, `min`, and `max` apply to `geo_2d`.

#### Validation

Use `#[validate(...)]` to attach field-level validation rules directly to your model fields.

These validators are checked before persistence, helping you catch invalid data close to the model itself. OxiMod also performs compile-time checks to prevent validators from being applied to incompatible field types. For example, string validators are restricted to string-like fields, numeric validators to numeric fields, and `required` to `Option<T>` fields.

```rust
#[validate(...)]
```

##### Length

These validators apply to types with a length, such as `String`, `Vec<T>`, arrays, and map/set types.

| Validator | Description |
|----------|------------|
| `min_length = N` | Requires the value length to be at least `N`. |
| `max_length = N` | Requires the value length to be at most `N`. |
| `non_empty` | Requires the value to contain at least one element or character. Equivalent to `min_length = 1`. |

##### String

These validators apply to string-like fields.

| Validator | Description |
|----------|------------|
| `starts_with = "..."` | Requires the string to start with the given prefix. |
| `ends_with = "..."` | Requires the string to end with the given suffix. |
| `includes = "..."` | Requires the string to contain the given substring. |
| `alphanumeric` | Restricts the value to ASCII letters and digits only. |
| `email` | Requires the value to match a basic email format. |
| `pattern = "..."` | Validates the value against a custom regular expression. |

##### Numeric

These validators apply to numeric fields. OxiMod supports inclusive range bounds through `min` and `max`, and also supports exclusive bounds with `min_exclusive` and `max_exclusive`.

| Validator | Description |
|----------|------------|
| `min = N` / `max = N` | Sets inclusive lower and upper bounds for the value. |
| `min_exclusive` / `max_exclusive` | Makes `min` strictly greater than and/or `max` strictly less than. |
| `positive` | Requires the value to be greater than `0`. |
| `negative` | Requires the value to be less than `0`. |
| `non_negative` | Requires the value to be greater than or equal to `0`. |
| `non_positive` | Requires the value to be less than or equal to `0`. |

##### Integer

These validators apply only to integer fields.

| Validator | Description |
|----------|------------|
| `multiple_of = N` | Requires the value to be evenly divisible by `N`. |

##### Optional

These validators apply to `Option<T>` fields.

| Validator | Description |
|----------|------------|
| `required` | Rejects `None` and requires the field to contain `Some(...)`. |

##### Custom

You can also provide your own validator function:

```rust
#[validate(custom(fn_name))]
```

Custom validators run after the built-in validations. The function receives a reference to the validated field type and must return `Result<(), String>`. For optional fields, OxiMod validates the inner type, so a custom validator on `Option<String>` receives `&String`, not `&Option<String>`. This keeps custom validation ergonomic and flexible.

```rust
fn validate_name(value: &String) -> Result<(), String> {
    if value == "admin" {
        return Err("reserved name".into());
    }
    Ok(())
}
```

Because your validator receives a reference to the field's effective validated type, you can keep the function focused on the actual value being checked. This works naturally alongside OxiMod's builder conversions and typed model fields.

##### Example

```rust
#[derive(Model)]
struct User {
    #[validate(required, min_length = 3, max_length = 30)]
    username: Option<String>,

    #[validate(email)]
    email: String,

    #[validate(non_negative, max = 100)]
    score: i64,

    #[validate(custom(validate_name))]
    name: String,
}
```

#### Defaults

Use `#[default(...)]` to assign a default value to a field when it is not explicitly set through the builder.

This allows you to define fallback values directly in your model, keeping initialization logic simple and centralized.

```rust
#[default(...)]
```

When a field is not provided during construction, OxiMod automatically applies the specified default before validation and persistence.

##### Examples

```rust
#[default("Guest")]
name: String,

#[default(42)]
score: i32,

#[default(false)]
active: bool,
```

##### Behavior

- **Applied during model construction**  
  Defaults are applied if the field is not set via the builder API.

- **Works with builder setters (`Into<T>`)**  
  Since setters accept `Into<T>`, defaults integrate seamlessly with flexible inputs (e.g., `&str` → `String`).

- **Supports optional and required fields**  
  - For required fields, the default acts as a fallback value.  
  - For `Option<T>`, defaults can still be applied if no value is provided.

- **Type-safe at compile time**  
  The default value must match the field type, ensuring correctness at compile time.

##### Example

```rust
#[derive(Model)]
struct User {
    #[default("Guest")]
    name: String,

    #[default(false)]
    active: bool,
}
```

```rust
let user = User::new();
// name = "Guest", active = false
```

##### Notes

- Defaults reduce boilerplate by eliminating the need to manually initialize common values.
- They are applied before validation, so validation rules still apply to defaulted values.

#### Hooks

```rust
#[hooks]
```

Hooks provide lifecycle extension points for model operations such as saving, querying, updating, and deleting documents.

They allow you to inject custom logic directly into your model workflows without modifying the core database logic.

Hooks are optional and must be enabled at the struct level using `#[hooks]`.

##### Save Hooks

| Hook | Signature | Description |
|----------|-----------|------------|
| `pre_save` | `async fn pre_save(&self) -> Result<(), OxiModError>` | Runs before `save()`. Use for validation or checks without mutation. |
| `post_save` | `async fn post_save(&self) -> Result<(), OxiModError>` | Runs after `save()`. Useful for logging or side effects. |
| `pre_save_mut` | `async fn pre_save_mut(&mut self) -> Result<(), OxiModError>` | Runs before `save_mut()`. Allows modifying the model before persistence. |
| `post_save_mut` | `async fn post_save_mut(&mut self) -> Result<(), OxiModError>` | Runs after `save_mut()`. Can modify in-memory state (not auto-persisted). |

##### Query Hooks

| Hook | Signature | Description |
|----------|-----------|------------|
| `pre_find` | `async fn pre_find(id: ObjectId) -> Result<(), OxiModError>` | Runs before `find_by_id()`. Useful for access control or logging. |
| `post_find` | `async fn post_find(result: &Option<Self>) -> Result<(), OxiModError>` | Runs after `find_by_id()`. Allows inspection of the fetched result. |

##### Mutation Hooks

| Hook | Signature | Description |
|----------|-----------|------------|
| `pre_update` | `async fn pre_update(id: ObjectId, update: &Document) -> Result<(), OxiModError>` | Runs before `update_by_id()`. Can abort based on custom logic. |
| `post_update` | `async fn post_update(id: ObjectId, update: &Document) -> Result<(), OxiModError>` | Runs after `update_by_id()`. Useful for logging or events. |
| `pre_delete` | `async fn pre_delete(id: ObjectId) -> Result<(), OxiModError>` | Runs before `delete_by_id()`. Can prevent deletion. |
| `post_delete` | `async fn post_delete(id: ObjectId) -> Result<(), OxiModError>` | Runs after `delete_by_id()`. Useful for cleanup or logging. |

##### Behavior

- **Opt-in feature**  
  Hooks are only generated when `#[hooks]` is present.

- **Model-level only**  
  Hooks run only for OxiMod model APIs (not raw collections).

- **Default no-op**  
  All hooks default to `Ok(())`; implement only what you need.

- **Error handling**  
  - Pre-hook errors abort operations  
  - Post-hook errors indicate post-processing failure


##### Example

```rust
use oximod::{Hooks, Model};

#[derive(Model)]
#[db("app")]
#[collection("logs")]
#[hooks]
struct Log {
    message: String,
}

#[async_trait::async_trait]
impl Hooks for Log {
    async fn pre_save(&self) -> Result<(), oximod::OxiModError> {
        println!("Saving log");
        Ok(())
    }

    async fn pre_save_mut(&mut self) -> Result<(), oximod::OxiModError> {
        self.message = self.message.trim().to_string();
        Ok(())
    }
}
```

---

## License

MIT

