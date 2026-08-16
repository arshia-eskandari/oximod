# OxiMod

<p align="center">
  <strong>Schema-aware MongoDB modeling and typed queries for Rust</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/oximod"><img src="https://img.shields.io/crates/v/oximod?cacheSeconds=300" alt="Crates.io"></a>
  <a href="https://docs.rs/oximod"><img src="https://docs.rs/oximod/badge.svg" alt="Documentation"></a>
  <a href="https://crates.io/crates/oximod"><img src="https://img.shields.io/crates/d/oximod" alt="Downloads"></a>
  <a href="https://github.com/arshia-eskandari/oximod/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT License"></a>
</p>

---

## What is OxiMod?

OxiMod is a schema-aware modeling layer built on top of the official MongoDB
Rust driver. It adds model-oriented ergonomics—derive-generated construction,
validation, defaults, indexes, lifecycle hooks, persistence helpers, and typed
queries—without hiding MongoDB or restricting access to the driver.

OxiMod is best understood as:

> **MongoDB with stronger model ergonomics, not a replacement for the driver.**

Use OxiMod when you want concise, expressive model code and compile-time
guidance for common MongoDB workflows. Where the driver is the better tool —
compound indexes, cross-namespace bulk writes, cursor streaming, advanced
session features — OxiMod keeps `mongodb::Collection`, raw BSON, and raw
aggregation pipelines directly accessible. Those escape hatches are supported
usage, not workarounds.

## Features

Each feature is covered in depth by a chapter of the
[OxiMod Guide](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/SUMMARY.md):

| Feature | What it provides | Guide |
| --- | --- | --- |
| Models & builders | One derive for collection-backed and embedded models, fluent setters, expression defaults | [Collection and Embedded Models](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/models/collection-and-embedded.md), [Builders, Defaults, and IDs](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/models/builders-defaults-and-ids.md) |
| Validation | Aggregated built-in, custom, and opt-in nested validation | [Validation](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/models/validation.md) |
| Persistence & clients | Save/find/update/delete helpers with global-client, explicit-client, and session-aware forms | [Persistence and Clients](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/persistence-and-clients.md) |
| Typed queries | Compile-time-checked filters, sorting, pagination, arrays, embedded paths, text and geospatial search | [Typed Queries](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/typed-queries.md) |
| Typed updates & deletion | Single- and multi-document typed writes with explicit modifier rules | [Updates and Deletion](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/updates-and-deletion.md) |
| Aggregation | Ordered pipeline builder mixing typed stages, raw stages, and typed output | [Aggregation](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/aggregation.md) |
| Sessions & transactions | Explicit `_with_session` counterparts that join a caller's `ClientSession` | [Sessions and Transactions](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/sessions-and-transactions.md) |
| Bulk writes | Model-scoped batches of mixed writes submitted as one driver bulk-write action | [Bulk Writes](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/operations/bulk-writes.md) |
| Indexes | Declarative single-field indexes with lazy or explicit establishment | [Declaring Indexes](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/indexes/declarations.md), [Establishment and Lifecycle](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/indexes/lifecycle.md) |
| Index drift & reconciliation | Read-only drift inspection and conservative create-only reconciliation | [Drift Detection and Reconciliation](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/indexes/reconciliation.md) |
| Lifecycle hooks | Optional save and `_id`-helper hooks with clear boundaries | [Lifecycle Hooks](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/advanced/hooks.md) |
| Direct driver access | Typed and raw `mongodb::Collection` escape hatches | [Direct Driver Access](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/advanced/driver-access.md) |
| Errors & boundaries | Failure-class error variants and OxiMod's behavioral boundaries in one place | [Errors and Behavioral Boundaries](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/advanced/errors-and-boundaries.md) |

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

## Quick start

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

The [Getting Started](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/getting-started.md)
chapter walks through this example step by step.

## Documentation

* [The OxiMod Guide](https://github.com/arshia-eskandari/oximod/blob/main/docs/src/SUMMARY.md)
  — long-form concepts, workflows, semantics, and boundaries.
* [API documentation on docs.rs](https://docs.rs/oximod) — precise public API
  contracts.
* [Runnable examples](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples)
  — complete workflows; run one with
  `cargo run -p oximod --example typed_query`. MongoDB-backed examples read
  `MONGODB_URI` from the environment or a `.env` file.
* [Contributing](https://github.com/arshia-eskandari/oximod/blob/main/CONTRIBUTING.md)
  — bug reports, documentation improvements, and pull requests are welcome.

## License

OxiMod is licensed under the [MIT License](https://github.com/arshia-eskandari/oximod/blob/main/LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in OxiMod shall be licensed under the MIT License without
additional terms or conditions.
