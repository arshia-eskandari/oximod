# Getting Started

## Installation

Add OxiMod and the dependencies used by your models and async runtime:

```bash
cargo add oximod mongodb@3.8.0
cargo add serde --features derive
cargo add tokio --features macros,rt-multi-thread
```

Add `async-trait` when implementing lifecycle hooks:

```bash
cargo add async-trait
```

A MongoDB server and connection URI are required for persistence and query
execution. OxiMod requires Rust 1.88 or newer and targets MongoDB Rust driver
3.8.0 or newer compatible 3.x releases.

## First model

Collection-backed models require a database name and collection name.
Importing `oximod::Model` brings both the derive macro and the runtime
persistence trait into scope; Rust keeps macro and type namespaces separate,
so the shared name is intentional.

```rust
use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiClient, Queryable};
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

    #[default(true)]
    active: bool,
}
```

The derive generates a fluent builder, a `validate()` method, the `Model`
persistence API, and the `Queryable` typed-query API for this struct. The
attributes used here are covered in depth by later chapters and summarized in
the [Attributes reference](reference/attributes.md).

## Connect, save, and query

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    OxiClient::init_global(
        "mongodb://localhost:27017".to_string(),
    )
    .await?;

    let user_id = User::new()
        .email("user1@example.com")
        .name("User1")
        .age(30)
        .save()
        .await?;

    let active_adults = User::query()
        .filter(|user| {
            user.active.eq(true) & user.age.gte(18)
        })
        .sort_by(|user| user.name.asc())
        .limit(20)
        .all()
        .await?;

    let saved_user = User::find_by_id(user_id).await?;

    println!("Found {} active adults", active_adults.len());
    println!("Saved user: {saved_user:#?}");

    Ok(())
}
```

Three things happen here:

1. `OxiClient::init_global(...)` installs the process-wide MongoDB client that
   ordinary model methods and typed queries execute through. It is a one-time
   startup step; see
   [Persistence and Clients](operations/persistence-and-clients.md).
2. `save()` validates the model, lazily establishes its declared indexes on
   first insertion, and returns the inserted `ObjectId`.
3. `User::query()` builds a typed query: every field exposes only the
   operations its Rust type supports, so incompatible filters fail to compile.
   See [Typed Queries](operations/typed-queries.md).

## Where to go next

* [Collection and Embedded Models](models/collection-and-embedded.md) — the
  two model kinds and what each receives.
* [Validation](models/validation.md) — built-in rules, custom validators, and
  nested validation.
* [Runnable Examples](reference/examples.md) — complete workflows you can run
  against a real MongoDB deployment.
