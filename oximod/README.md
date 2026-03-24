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
- validation and defaults  
- index declarations  
- typed model helpers  
- global and explicit-client workflows  
- optional lifecycle hooks  

At the same time, it preserves MongoDB’s native power by exposing:

- `mongodb::Collection<Self>`
- `mongodb::Collection<Document>`

OxiMod is best understood as:

> **MongoDB with stronger model ergonomics**, not a replacement for the driver.

---

## Design Philosophy

OxiMod is intentionally lightweight.

It focuses on areas that benefit from schema-awareness:

- model definition  
- builder construction  
- validation  
- defaults  
- index setup  
- optional lifecycle hooks  

For everything else, use the MongoDB driver directly:

- `Model::get_collection()`
- `Model::get_document_collection()`

This ensures:
- zero feature lock-in  
- full MongoDB flexibility  
- long-term maintainability  

---

## Builder API

```rust
let user = User::new()
    .name("Alice")
    .age(30)
    .active(true);
```

### Features

- accepts any `Into<T>`
- automatic conversions
- applies defaults
- supports optional + required fields
- customizable `_id` setter

---

## Model API

### Core

| Method | Description |
|------|------------|
| `save()` | Insert document |
| `save_mut()` | Insert document with mutable hooks |
| `clear()` | Remove all documents |
| `get_collection()` | Typed collection |
| `get_document_collection()` | Raw collection |

### Identity Helpers

| Method | Description |
|------|------------|
| `find_by_id()` | Fetch by `_id` |
| `update_by_id()` | Update by `_id` |
| `delete_by_id()` | Delete by `_id` |

### Utilities

| Method | Description |
|------|------------|
| `exists()` | Check existence |
| `count()` | Count documents |

---

## Client Usage

### Global

```rust
OxiClient::init_global(uri).await?;
user.save().await?;
```

### Explicit

```rust
user.save_from(&client).await?;
```

Used for:
- tests  
- multi-tenant apps  
- dependency injection  

---

## Collections

### Typed

```rust
let collection = User::get_collection()?;
```

### Raw

```rust
let collection = User::get_document_collection()?;
```

---

## Attributes

### Struct-Level

| Attribute | Description |
|----------|------------|
| `#[db("name")]` | Database |
| `#[collection("name")]` | Collection |
| `#[document_id_setter_ident("name")]` | Rename `_id` setter |
| `#[index_max_retries(N)]` | Retry count |
| `#[index_max_init_seconds(N)]` | Timeout |
| `#[hooks]` | Enable lifecycle hooks |

---

### Indexing

```rust
#[index(...)]
```

#### Core

| Attribute | Description |
|----------|------------|
| `unique` | Unique index |
| `sparse` | Skip missing |
| `hidden` | Hide index |
| `name = "..."` | Custom name |
| `order = 1/-1` | Sort order |
| `expire_after_secs` | TTL |

#### Advanced Types

| Attribute | Description |
|----------|------------|
| `text` | Text index |
| `hashed` | Hashed index |
| `geo_2dsphere` | Geo index |

#### Advanced Options

| Attribute | Description |
|----------|------------|
| `version` | Index version |
| `text_index_version` | Text version |
| `geo_2dsphere_index_version` | Geo version |
| `weight` | Text weight |
| `default_language` | Text language |
| `case_insensitive` | Collation |

---

## Validation

```rust
#[validate(...)]
```

### Length

| Validator | Description |
|----------|------------|
| `min_length` | Minimum |
| `max_length` | Maximum |
| `non_empty` | Not empty |

### String

| Validator | Description |
|----------|------------|
| `starts_with` | Prefix |
| `ends_with` | Suffix |
| `includes` | Contains |
| `alphanumeric` | ASCII |
| `email` | Email |
| `pattern` | Regex |

### Numeric

| Validator | Description |
|----------|------------|
| `min` / `max` | Range |
| `positive` | > 0 |
| `negative` | < 0 |
| `non_negative` | ≥ 0 |
| `non_positive` | ≤ 0 |

### Integer

| Validator | Description |
|----------|------------|
| `multiple_of` | Divisible |

### Optional

| Validator | Description |
|----------|------------|
| `required` | Not None |

### Custom

```rust
#[validate(custom(fn_name))]
```

---

## Defaults

```rust
#[default(...)]
```

Examples:

- `#[default("Guest".to_string())]`
- `#[default(42)]`
- `#[default(false)]`

---

## Hooks

```rust
#[hooks]
```

### Save Hooks

| Hook | Description |
|----------|------------|
| `pre_save` | Runs before `save()` |
| `post_save` | Runs after `save()` |
| `pre_save_mut` | Runs before `save_mut()` |
| `post_save_mut` | Runs after `save_mut()` |

### Query Hooks

| Hook | Description |
|----------|------------|
| `pre_find` | Runs before `find_by_id()` |
| `post_find` | Runs after `find_by_id()` |

### Mutation Hooks

| Hook | Description |
|----------|------------|
| `pre_update` | Runs before `update_by_id()` |
| `post_update` | Runs after `update_by_id()` |
| `pre_delete` | Runs before `delete_by_id()` |
| `post_delete` | Runs after `delete_by_id()` |

Hooks are optional and are enabled at the struct level with `#[hooks]`.

Hooks are implemented by implementing the `Hooks` trait for the model.

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
}
```

Hooks are useful for:

- normalization
- logging
- validation beyond schema rules
- audit trails
- business rules
- event emission

---

## Example

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

    #[index(unique, name = "email_idx")]
    #[validate(email)]
    email: String,

    #[validate(min_length = 3, max_length = 32)]
    name: String,

    #[validate(non_negative)]
    age: i32,

    #[default(false)]
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize global client
    dotenv::dotenv().ok();
    let uri = std::env::var("MONGODB_URI")?;
    OxiClient::init_global(uri).await?;

    // Clear collection
    User::clear().await?;

    // Build model using builder API
    let user = User::new()
        .email("alice@example.com")
        .name("Alice")
        .age(30)
        .active(true);

    // Save document
    let id = user.save().await?;
    println!("Inserted user: {}", id);

    // Find by id
    if let Some(found) = User::find_by_id(id).await? {
        println!("Found user: {}", found.name);
    }

    // Count documents
    let count = User::count(doc! {}).await?;
    println!("Total users: {}", count);

    // Use MongoDB driver directly
    let collection = User::get_collection()?;

    collection
        .update_one(
            doc! { "_id": id },
            doc! { "$set": { "active": false } },
        )
        .await?;

    println!("User updated");

    Ok(())
}
```

For more examples, feel free to check out the [`examples/`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples) directory.

---

## Philosophy Summary

- minimal abstraction  
- maximum flexibility  
- compile-time safety  
- production-ready ergonomics  

---

## License

MIT
