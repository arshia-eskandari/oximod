# OxiMod

**A MongoDB ODM for Rust**

---

## Overview

OxiMod is a schema-based Object-Document Mapper (ODM) for MongoDB, designed for Rust developers who want a familiar and expressive way to model and interact with their data.

Inspired by Mongoose, OxiMod brings a structured modeling experience while embracing Rust's type safety and performance. It works with any async runtime and is currently tested using `tokio`.

### 🚀 Highlighted Feature (since `v0.1.7`) – Fluent API Builders

OxiMod now supports **fluent API builders** and a `new()` method for ergonomic model creation:

```rust
let user = User::new()
    .name("Alice".to_string())
    .age(30)
    .active(true);
```

- Supports `Option<T>` and non-optional fields.
- Works with `#[default("...")]` for seamless defaults.
- Customize `_id` setter via `#[document_id_setter_ident("...")]`.

Use `user.save().await?` just like before!

---

### 🆕 Improvements in `v0.1.12`

✅ Removed the `aggregate` method.  
   - Instead, OxiMod provides `get_collection()` for direct access to the underlying MongoDB collection.  
   - This allows users to perform advanced operations such as `aggregate`, `bulk_write`, and `insert_many` directly with the MongoDB driver, reducing macro expansion complexity and improving maintainability.
✅ Added index initialization controls:
   - `#[index_max_retries(N)]` → Maximum retry attempts when creating indexes.
   - `#[index_max_init_seconds(N)]` → Maximum time allowed for index initialization.
   - These work with OxiMod’s internal `OnceAsync` system to ensure indexes are created once per process, with resilience against transient failures and protection against indefinite hangs.
✅ Updated `alphanumeric` validator to strictly check against **ASCII alphanumeric characters**.  
✅ Improved validation safety by preventing impossible cases (e.g., defining a `min` greater than a `max`).  
✅ Performance optimizations verified using **flamegraph profiling**.  
   - Reduced unnecessary resource allocations.  
   - Improved execution efficiency across common operations.  
✅ General reliability improvements and bug fixes.

If you encounter any bugs or have suggestions, **please open an issue on GitHub to report them** — your feedback helps improve OxiMod for everyone.

---

## Features

- **Schema Modeling with Macros**  
  Define your collections using idiomatic Rust structs and a simple derive macro.

- **Async-Friendly**  
  Built for asynchronous Rust. Integrates seamlessly with the `mongodb` driver.

- **Built-in CRUD Operations**  
  Use `save()`, `find()`, `update()`, `delete()`, and more directly on your types.

- **Direct Collection Access**  
  Use `get_collection()` to retrieve the underlying MongoDB collection.  
  This enables direct access to advanced MongoDB features such as `aggregate`, `bulk_write`, and `insert_many`.

- **Minimal Boilerplate**  
  Declare a model in seconds with `#[derive(Model)]`, `#[db]`, and `#[collection]` attributes.

- **Indexing Support**  
  Add indexes declaratively via field-level `#[index(...)]` attributes.

- **Validation Support**  
  Add field-level validation using `#[validate(...)]`. Supports length, email, pattern, positivity, and more.  
  - Now includes **ASCII alphanumeric** validation.  
  - Prevents invalid validator combinations (e.g., `min > max`).

- **Default Values**  
  Use `#[default(...)]` to specify field defaults for strings, numbers, and enums.

- **Builder API & `new()` Support**  
  Use `Model::default()` or `Model::new()` to initialize structs and chain fluent setters. Customize `_id` setter name with `#[document_id_setter_ident(...)]`.

- **Clear Error Handling**  
  Strongly typed, developer-friendly errors based on `thiserror`. Includes optional debugging output with `backtrace` and human-readable suggestions when used with `RUST_BACKTRACE=full`.

---

## Model Attributes

OxiMod supports attributes at both the struct level and field level.

### Struct-Level Attributes

- `#[db("name")]`: Specifies the MongoDB database the model belongs to.
- `#[collection("name")]`: Specifies the collection name within the database.
- `#[document_id_setter_ident("name")]`: Optional. Renames the `_id` builder function for fluent `.new()`/`.default()` APIs.
- `#[index_max_retries(N)]` – (optional) Maximum number of times OxiMod will retry creating indexes when a model is first used.
    - Powered by the internal `OnceAsync` primitive, which ensures indexes are created once per process.
    - If index creation fails (e.g. transient network hiccups), OxiMod will retry up to `N` times before surfacing an error.
- `#[index_max_init_seconds(N)]` – (optional) Maximum time allowed for index initialization during the first attempt.
    - Prevents “stuck” initializations if MongoDB is unresponsive or index creation is unusually slow.
    - After `N` seconds, the initializer aborts and future operations can retry, instead of leaving your app hanging indefinitely.

### Field-Level Index Attributes

You can add indexes to fields using the `#[index(...)]` attribute.

#### Supported Options:

- `unique`: Ensures values in this field are unique.
- `sparse`: Indexes only documents that contain the field.
- `name = "..."`: Custom name for the index.
- `background`: Builds index in the background without locking the database.
- `order = 1 | -1`: Index sort order (1 = ascending, -1 = descending).
- `expire_after_secs = ...`: Time-to-live for the index in seconds.
- `version = N`: Specifies the index version (e.g., 1, 2, etc.).
- `text_index_version = N`: Specifies the text index version (1, 2, 3, etc.). Use only with text indexes.
- `hidden`: If true, the index is created but hidden from the query planner by default.

### Field-Level Validation Attributes

You can apply validations on fields using the `#[validate(...)]` attribute.

#### Supported Validators:

- `min_length = N`: Minimum length for `String` values.
- `max_length = N`: Maximum length for `String` values.
- `required`: Ensures the field is not `None`.
- `email`: Validates the format of an email.
- `pattern = "regex"`: Validates the value against a regex pattern.
- `non_empty`: Ensures a `String` is not empty or whitespace.
- `positive`: Ensures numeric value is greater than 0.
- `negative`: Ensures numeric value is less than 0.
- `non_negative`: Ensures numeric value is 0 or greater.
- `min = N`: Ensures numeric value is at least `N`.
- `max = N`: Ensures numeric value is at most `N`.
- `starts_with = "..."`: Ensures a string starts with the given prefix.
- `ends_with = "..."`: Ensures a string ends with the given suffix.
- `includes = "..."`: Ensures a string includes the given substring.
- `alphanumeric`: Ensures all characters are alphanumeric (ASCII).
- `multiple_of = N`: Ensures the numeric value is a multiple of `N`.

> 💡 Use native Rust enums instead of `enum_values`.

### Field-Level Default Attributes

- `#[default("value")]`: Assigns a default value for strings.
- `#[default(42)]`: Sets default for numbers.
- `#[default(MyEnum::Variant)]`: Sets default for enums.

These defaults are applied when using `Model::new()` or `Model::default()`.

---

## Example

```rust
use oximod::{set_global_client, Model};
use serde::{Serialize, Deserialize};
use mongodb::bson::{doc, oid::ObjectId};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "email_idx", order = -1)]
    #[validate(email)]
    email: String,

    #[index(sparse)]
    phone: Option<String>,

    #[validate(min_length = 3)]
    name: String,

    #[validate(non_negative)]
    age: i32,

    #[validate(non_negative, multiple_of = 5)]
    points: i32,

    #[index(hidden)]
    internal_tag: String,

    #[default(false)]
    active: bool,
}

/// Initialize MongoDB connection from `.env`
async fn init() -> Result<()> {
    dotenvy::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("Missing MONGODB_URI in .env");
    set_global_client(mongodb_uri).await?;
    Ok(())
}

/// Create and save a user
async fn save_user() -> Result<()> {
    let user = User::new()
        .email("alice@example.com".to_string())
        .name("Alice".to_string())
        .age(30)
        .points(50)
        .internal_tag("internal_use".to_string())
        .active(true);

    user.save().await?;
    println!("✅ User saved: {:?}", user);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init().await?;
    save_user().await?;
    Ok(())
}
```

In this example:

- `#[db("my_app_db")]` and `#[collection("users")]` configure the database and collection.
- The `email` field has a descending, unique index with a custom name and must contain a valid email.
- The `phone` field is indexed only when it exists in the document (sparse).
- The `name` field must be at least 3 characters long.
- The `age` field must be non-negative.
- The `points` field must be non-negative and a multiple of 5.
- The `internal_tag` field is indexed but hidden from query planning, useful for internal/system metadata. 
- The `active` field defaults to `false`.

---
## Running Examples

OxiMod includes a growing set of usage examples:

```bash
cargo run --example basic_usage
cargo run --example aggregate_usage
cargo run --example validate_usage
cargo run --example query
cargo run --example update
cargo run --example delete
cargo run --example hook_usage
cargo run --example by_id
cargo run --example default_usage
```

Each file clears previous data on run and demonstrates isolated functionality.

> Don't forget to create a `.env` file:
>
> ```env
> MONGODB_URI=mongodb://localhost:27017
> ```

---

## Contributing & Feedback

We welcome all contributions, suggestions, and feedback!  
If you discover a bug or want to request a feature, **please open an issue on GitHub**.  
Your input helps improve OxiMod for everyone — thank you for your support.

---

## License

[MIT](./LICENSE) © 2025 OxiMod Contributors

> ⚠️ The name **OxiMod** and this repository represent the official version of the project.  
> Forks are welcome, but please **do not use the name or create similarly named organizations** to avoid confusion with the original.

---

We hope OxiMod helps bring joy and structure to your MongoDB experience in Rust.

Contributions welcome!
