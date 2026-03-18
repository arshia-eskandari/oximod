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

## Example

```rust
#[derive(Model)]
#[db("app")]
#[collection("users")]
struct User {
    #[index(unique)]
    #[validate(email)]
    email: String,

    #[validate(min_length = 3)]
    name: String,

    #[default(false)]
    active: bool,
}
```

---

## Philosophy Summary

- minimal abstraction  
- maximum flexibility  
- compile-time safety  
- production-ready ergonomics  

---

## License

MIT
