# OxiMod

**A MongoDB ODM for Rust**

---

## Overview

OxiMod is a schema-based Object-Document Mapper (ODM) for MongoDB, designed for Rust developers who want a familiar and expressive way to model and interact with their data.

Inspired by Mongoose, OxiMod brings a structured modeling experience while embracing Rust's type safety and performance. It works with any async runtime and is currently tested using `tokio`.

---

## 🚀 Fluent API Builders (since `v0.1.7`)

OxiMod supports `new()` and fluent builder-style setters:

```rust
let user = User::new()
    .name("Alice".to_string())
    .age(30)
    .active(true);
```

- Works with `Option<T>` and non-option field  
- Uses `#[default("...")]` when defined  
- Supports renaming the `_id` setter via `#[document_id_setter_ident("...")]`  

Use:

```rust
user.save().await?;
```

---

## 🆕 Improvements in `v0.1.12`

### Index Initialization Controls
- `#[index_max_retries(N)]`
- `#[index_max_init_seconds(N)]`

These provide robust, retry-aware index creation using OxiMod’s internal `OnceAsync`.

### Validation Improvements
- `alphanumeric` now checks ASCII-only  
- Prevents contradictory validators (e.g., min > max)

### Performance Enhancements
- Flamegraph analysis  
- Fewer allocations  
- Optimized execution paths  

---

## 🆕 OxiClient – Global & Multi-Client Support

OxiMod now ships with a MongoDB client wrapper: **`OxiClient`**.

This enables:

### ✔️ Global client initialization  
### ✔️ Multiple clients for multi-tenant / multi-database setups  
### ✔️ Fully client-aware CRUD APIs  

---

### Initializing the Global Client (short pattern)

```rust
use oximod::OxiClient;

dotenv::dotenv().ok();
let mongodb_uri = std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
OxiClient::init_global(mongodb_uri).await?;
```

Once set, any `Model::save()`, `Model::find()`, etc. will automatically use the global client.

---

## 🆕 Passing Clients Manually — `*_with_client` Methods

Every CRUD operation now has a **client-aware variant**:

| Purpose | Auto Client | Manual Client |
|--------|-------------|----------------|
| Insert | `save()` | `save_with_client(&Client)` |
| Update | `update()` | `update_with_client(_, _, &Client)` |
| Query | `find()` | `find_with_client(_, &Client)` |
| Find One | `find_one()` | `find_one_with_client(_, &Client)` |
| Delete | `delete()` | `delete_with_client(_, &Client)` |
| By ID | `find_by_id()` | `find_by_id_with_client(_, &Client)` |
| Clear | `clear()` | `clear_with_client(&Client)` |

Useful for:

- Multi-tenant systems  
- Test isolation  
- Cluster routing  
- Advanced architectures  

---

## 🆕 New Example: `update_with_client`

Located in:

```
examples/update_with_client.rs
```

Demonstrates:

- Using `OxiClient::new()`  
- Using `*_with_client` CRUD variants  
- Updating documents with explicit clients  

---

## Features

- Schema Modeling with Macros  
- Async-friendly (Tokio tested)  
- Built‑in CRUD operations  
- Direct `get_collection()` access  
- Indexing support (`#[index(...)]`)  
- Validation support (`#[validate(...)]`)  
- Default values (`#[default(...)]`)  
- Fluent builder API  
- Clear and typed error handling  

---

## Attributes

### Struct-Level

- `#[db("name")]`
- `#[collection("name")]`
- `#[document_id_setter_ident("name")]`
- `#[index_max_retries(N)]`
- `#[index_max_init_seconds(N)]`

### Field-Level (Indexing)

`#[index(unique, sparse, name = "...", order = 1 | -1, hidden, expire_after_secs = N, ...)]`

### Field-Level (Validation)

- `min_length`, `max_length`
- `required`
- `email`
- `pattern = "regex"`
- `positive`, `negative`, `non_negative`
- `min = N`, `max = N`
- `starts_with`, `ends_with`, `includes`
- `alphanumeric`
- `multiple_of`

### Defaults

`#[default("string")]`, `#[default(42)]`, `#[default(Enum::Variant)]`

---

## Example Usage

```rust
use oximod::{Model, OxiClient};
use serde::{Serialize, Deserialize};
use mongodb::bson::{doc, oid::ObjectId};
use anyhow::Result;

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
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let mongodb_uri =
        std::env::var("MONGODB_URI").expect("Missing MONGODB_URI");
    OxiClient::init_global(mongodb_uri).await?;

    let user = User::new()
        .email("alice@example.com".to_string())
        .name("Alice".to_string())
        .age(30)
        .active(true);

    let id = user.save().await?;
    println!("Inserted user: {:?}", id);

    let found = User::find_by_id(id).await?;
    println!("Found: {:?}", found);

    Ok(())
}
```

---

## Running Examples

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

Ensure:

```env
MONGODB_URI=mongodb://localhost:27017
```

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
