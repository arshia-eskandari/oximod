# OxiMod

**A MongoDB ODM for Rust**

---

## Overview

OxiMod is a schema-based Object-Document Mapper (ODM) for MongoDB, designed for Rust developers who want a familiar and expressive way to model and interact with their data.

Inspired by Mongoose, OxiMod brings a structured modeling experience while embracing Rust's type safety and performance. It works with any async runtime and is currently tested using `tokio`.

---

## Features

- **Schema Modeling with Macros**  
  Define your collections using idiomatic Rust structs and a simple derive macro.

- **Async-Friendly**  
  Built for asynchronous Rust. Integrates seamlessly with the `mongodb` driver.

- **Built-in CRUD Operations**  
  Use `save()`, `find()`, `update()`, `delete()`, and more directly on your types.

- **Minimal Boilerplate**  
  Declare a model in seconds with `#[derive(Model)]`, `#[db]`, and `#[collection]` attributes.

- **Indexing Support**  
  Add indexes declaratively via field-level `#[index(...)]` attributes.

- **Validation Support**  
  Add field-level validation using `#[validate(...)]`. Supports length, email, pattern, positivity, and more.

- **Clear Error Handling**  
  Strongly typed, developer-friendly errors based on `thiserror`.

---

## Model Attributes

OxiMod supports attributes at both the struct level and field level.

### Struct-Level Attributes

- `#[db("name")]`: Specifies the MongoDB database the model belongs to.
- `#[collection("name")]`: Specifies the collection name within the database.

### Field-Level Index Attributes

You can add indexes to fields using the `#[index(...)]` attribute.

#### Supported Options:

- `unique`: Ensures values in this field are unique.
- `sparse`: Indexes only documents that contain the field.
- `name = "...""`: Custom name for the index.
- `background`: Builds index in the background without locking the database.
- `order = 1 | -1`: Index sort order (1 = ascending, -1 = descending).
- `expire_after_secs = ...`: Time-to-live for the index in seconds.

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

> 💡 Use native Rust enums instead of `enum_values`.

---

## Example

This example demonstrates how to define a model with schema-level and field-level metadata.

```rust
use oximod::{set_global_client, Model};
use serde::{Serialize, Deserialize};
use mongodb::bson::{doc, oid::ObjectId};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[index(unique, name = "email_idx", order = -1)]
    email: String,

    #[index(sparse)]
    phone: Option<String>,

    #[validate(min_length = 3)]
    name: String,

    #[validate(non_negative)]
    age: i32,

    active: bool,
}
```

In this example:
- `#[db("my_app_db")]` and `#[collection("users")]` configure the database and collection.
- The `email` field has a descending, unique index with a custom name.
- The `phone` field is indexed only when it exists in the document (sparse).
- The `name` field must be at least 3 characters long.
- The `age` field must be non-negative.

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
cargo run --example by_id
```

Each file clears previous data on run and demonstrates isolated functionality.

> Don't forget to create a `.env` file:
>
> ```env
> MONGODB_URI=mongodb://localhost:27017
> ```

---

## License

[MIT](./LICENSE) © 2025 OxiMod Contributors

> ⚠️ The name **OxiMod** and this repository represent the official version of the project.  
> Forks are welcome, but please **do not use the name or create similarly named organizations** to avoid confusion with the original.

---

We hope OxiMod helps bring joy and structure to your MongoDB experience in Rust.

Contributions welcome!

