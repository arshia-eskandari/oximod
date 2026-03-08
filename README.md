# OxiMod

**A schema-aware MongoDB toolkit for Rust**

---

## Overview

OxiMod is a schema-based modeling layer for MongoDB, designed for Rust developers who want a more expressive way to define models without giving up direct access to the MongoDB driver.

Inspired by the productivity of ODM-style workflows, OxiMod adds:

- derive-based schema configuration
- builder-style model construction
- validation and defaults
- index declarations
- typed model helpers
- global and explicit-client workflows

At the same time, it intentionally preserves MongoDB's native power by exposing the underlying typed `mongodb::Collection<Self>` and raw `mongodb::Collection<Document>` when needed.

OxiMod is best understood as **MongoDB with stronger model ergonomics**, not as a replacement for the Rust MongoDB driver.

---

## Design Philosophy

OxiMod is intentionally lightweight.

Instead of wrapping every MongoDB operation behind a custom API, OxiMod focuses on the parts that benefit most from schema awareness:

- model definition
- builder-style construction
- validation
- default values
- index setup
- common identity-based helpers
- global or explicit-client model access

For general querying and advanced operations, you continue using the MongoDB driver directly through:

- `Model::get_collection()` / `Model::get_collection_from(...)`
- `Model::get_document_collection()` / `Model::get_document_collection_from(...)`

This means users keep full access to MongoDB features without waiting for OxiMod to mirror the entire driver surface.

---

## Builder API

OxiMod supports `new()` and fluent builder-style setters:

```rust
let user = User::new()
    .name("Alice")
    .age(30)
    .active(true);
```

Builder setters are flexible:

- any type implementing `Into<T>` for the field type can be passed directly
- conversions happen inside the setter
- `#[default(...)]` values are applied automatically
- both optional and non-optional fields are supported
- the `_id` setter can be renamed with `#[document_id_setter_ident("...")]`

### Example

```rust
let user = User::new()
    .name("Alice")   // &str -> String
    .age(30)
    .active(true);
```

Save with:

```rust
let id = user.save().await?;
```

---

## Model API

The `Model` trait is typically derived with `#[derive(Model)]`.

### Core operations

- `save()` / `save_from(...)`
- `clear()` / `clear_from(...)`
- `get_collection()` / `get_collection_from(...)`
- `get_document_collection()` / `get_document_collection_from(...)`

### Identity and utility helpers

- `find_by_id()` / `find_by_id_from(...)`
- `update_by_id()` / `update_by_id_from(...)`
- `delete_by_id()` / `delete_by_id_from(...)`
- `exists()` / `exists_from(...)`
- `count()` / `count_from(...)`

### Important note

OxiMod no longer attempts to wrap the entire MongoDB CRUD surface with custom `find`, `find_one`, `update`, `delete`, and similar methods.

Instead:

- use OxiMod helpers for high-value, schema-aware convenience
- use the MongoDB collection directly for general queries and updates

This keeps the library easier to maintain and prevents loss of driver functionality.

---

## Global and Explicit Client Usage

OxiMod supports two access patterns.

### 1. Global client

Initialize a shared client once:

```rust
use oximod::OxiClient;

dotenv::dotenv().ok();
let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
OxiClient::init_global(mongodb_uri).await?;
```

Then use convenience methods such as:

- `user.save().await?`
- `User::clear().await?`
- `User::find_by_id(id).await?`
- `User::count(doc! {}).await?`

### 2. Explicit client

For tests, scoped lifetimes, or multi-client environments, use the `*_from` methods:

- `save_from(&client)`
- `clear_from(&client)`
- `find_by_id_from(id, &client)`
- `update_by_id_from(id, update, &client)`
- `delete_by_id_from(id, &client)`
- `exists_from(filter, &client)`
- `count_from(filter, &client)`

You can also obtain collections explicitly with:

- `get_collection_from(&client)`
- `get_document_collection_from(&client)`

This pattern is well-suited for:

- test isolation
- multi-tenant systems
- multi-database architectures
- explicit dependency injection

---

## Typed and Raw Collections

One of OxiMod's core strengths is that it exposes both typed and raw collection access.

### Typed collection

```rust
let collection = User::get_collection()?;
let found = collection.find_one(doc! { "_id": id }).await?;
```

This returns a `mongodb::Collection<User>` and is the preferred option for most application code.

### Raw document collection

```rust
let collection = User::get_document_collection()?;
let found = collection.find_one(doc! { "_id": id }).await?;
```

This returns a `mongodb::Collection<Document>` and is useful when you want to work directly with BSON documents.

---

## Features

- Schema modeling via `#[derive(Model)]`
- Fluent builder API
- Global and explicit-client workflows
- Typed collection access
- Raw document collection access
- Identity helpers (`find_by_id`, `update_by_id`, `delete_by_id`)
- Utility helpers (`exists`, `count`)
- Validation support
- Default values
- Index support
- Clear, typed error handling
- Async-friendly design

OxiMod is tested with `tokio`, while remaining compatible with MongoDB driver workflows that fit broader async usage patterns.

---

## Attributes

### Struct-level attributes

- `#[db("name")]`
- `#[collection("name")]`
- `#[document_id_setter_ident("name")]`
- `#[index_max_retries(N)]`
- `#[index_max_init_seconds(N)]`

### Field-level indexing

```rust
#[index(unique, sparse, name = "...", order = 1 | -1, hidden, expire_after_secs = N, ...)]
```

### Field-level validation

Supported validators include:

- `min_length`
- `max_length`
- `required`
- `email`
- `pattern = "..."`
- `positive`
- `negative`
- `non_negative`
- `min = N`
- `max = N`
- `starts_with`
- `ends_with`
- `includes`
- `alphanumeric`
- `multiple_of`

### Defaults

Examples:

- `#[default("Guest".to_string())]`
- `#[default(42)]`
- `#[default(false)]`
- `#[default(Enum::Variant)]`

---

## Example Usage

```rust
use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique)]
    #[validate(email)]
    email: String,

    #[validate(min_length = 3)]
    name: String,

    #[validate(non_negative)]
    age: i32,

    #[default(false)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(mongodb_uri).await?;

    let user = User::new()
        .email("alice@example.com")
        .name("Alice")
        .age(30)
        .active(true);

    let id = user.save().await?;
    println!("Inserted user: {}", id);

    if let Some(found) = User::find_by_id(id).await? {
        println!("Found user: {}", found.name);
    }

    let count = User::count(doc! { "active": true }).await?;
    println!("Active users: {}", count);

    let exists = User::exists(doc! { "email": "alice@example.com" }).await?;
    println!("User exists: {}", exists);

    let collection = User::get_collection()?;
    let updated = collection
        .update_one(
            doc! { "_id": id },
            doc! { "$set": { "active": false } },
        )
        .await?;

    println!("Updated {} document(s)", updated.modified_count);

    Ok(())
}
```

---

## Examples

Current examples demonstrate both OxiMod helpers and raw MongoDB collection access:

```bash
cargo run --example basic_usage
cargo run --example validate_usage
cargo run --example query
cargo run --example update
cargo run --example update_with_client
cargo run --example delete
cargo run --example hook_usage
cargo run --example by_id
cargo run --example default_usage
```

Most examples intentionally show a mix of:

- OxiMod helpers for common model operations
- direct `Collection<T>` usage for general MongoDB workflows

This reflects the intended usage style of the library.

---

## Environment

Examples typically expect:

```env
MONGODB_URI=mongodb://localhost:27017
```

---

## Version Notes

### Builder ergonomics

Recent versions improved the builder API so setters accept any type implementing `Into<T>` for the field type.

### Index initialization controls

OxiMod includes:

- `#[index_max_retries(N)]`
- `#[index_max_init_seconds(N)]`

These help control retry-aware index initialization behavior for derived models.

### Validation improvements

Recent updates also improved validation behavior, including stronger handling around contradictory validators and stricter checks for specific validation kinds such as ASCII-only alphanumeric validation.

---

## Contributing and Feedback

Contributions, issues, ideas, and feedback are welcome.

If you discover a bug or want to request a feature, please open an issue on GitHub. Clear reports, reproduction steps, and concrete API suggestions are especially helpful.

---

## License

[MIT](./LICENSE) © 2025 OxiMod Contributors

> The name **OxiMod** and this repository represent the official version of the project.
> Forks are welcome, but please do not use the name or create similarly named organizations in ways that may cause confusion with the original project.

---

OxiMod aims to make MongoDB modeling in Rust more expressive without compromising the power of the underlying driver.
