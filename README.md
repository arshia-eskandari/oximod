# Monoxide

**A MongoDB ODM for Rust**

---

## Overview

Monoxide is a schema-based Object-Document Mapper (ODM) for MongoDB, designed for Rust developers who want a familiar and expressive way to model and interact with their data.

Inspired by Mongoose, Monoxide brings a structured modeling experience while embracing Rust's type safety and performance. It works with any async runtime and is currently tested using `tokio`.

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

- **Clear Error Handling**  
  Strongly typed, developer-friendly errors based on `thiserror`.

---

## Example

```rust
use monoxide_core::feature::conn::client::set_global_client;
use monoxide_core::feature::model::Model;
use monoxide_macros::Model;
use serde::{Serialize, Deserialize};
use mongodb::bson::{doc, oid::ObjectId};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("my_app_db")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,
    name: String,
    age: i32,
    active: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let mongodb_uri = std::env::var("MONGODB_URI")?;
    set_global_client(mongodb_uri).await?;

    let user = User {
        _id: None,
        name: "User1".into(),
        age: 29,
        active: true,
    };

    let id = user.save().await?;
    println!("Inserted user with _id: {}", id);

    Ok(())
}
```

---

## Running Examples

Monoxide includes a growing set of usage examples:

```bash
cargo run --example basic_usage
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

[MIT](./LICENSE) © 2025 Monoxide Contributors

---

We hope Monoxide helps bring joy and structure to your MongoDB experience in Rust.

Contributions welcome!

