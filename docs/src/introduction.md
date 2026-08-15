# Introduction

OxiMod is a schema-aware modeling layer built on top of the official MongoDB
Rust driver. It adds model-oriented ergonomics—derive-generated construction,
validation, defaults, indexes, lifecycle hooks, persistence helpers, and typed
queries—without hiding MongoDB or restricting access to the driver.

OxiMod is best understood as:

> **MongoDB with stronger model ergonomics, not a replacement for the driver.**

Use OxiMod when you want concise, expressive model code and compile-time
guidance for common MongoDB workflows, while retaining direct access to:

* `mongodb::Collection<Model>`;
* `mongodb::Collection<Document>`;
* raw BSON filters and updates;
* raw aggregation pipelines;
* sessions, compound indexes, and advanced driver options.

## What OxiMod provides

* One `Model` derive for collection-backed and embedded models
* Fluent generated builders with `Into<T>` setters
* Field defaults expressed as ordinary Rust expressions
* Aggregated built-in and custom validation
* Declarative single-field MongoDB indexes
* Explicit index drift inspection with conservative create-only reconciliation
* Global-client and explicit-client persistence workflows
* Type-aware filters, sorting, pagination, text search, and geospatial queries
* Typed single-document and multi-document updates and deletions
* Typed model-scoped bulk writes batching mixed operations into one driver
  bulk-write action
* First-class aggregation builder mixing typed stages, raw stages, and typed
  output
* Typed nested paths for embedded documents and arrays of embedded models
* Explicit session-aware operations for MongoDB transactions
* Optional lifecycle hooks for save and `_id` helper operations
* Structured validation and typed-query errors
* Full MongoDB driver escape hatches

## What OxiMod deliberately does not do

OxiMod does not replace MongoDB's own validation, transactions, index
enforcement, or retry behavior, and it does not wrap the entire driver API.
Where the driver is the better tool—compound indexes, cross-namespace bulk
writes, cursor streaming, advanced session features—OxiMod points you at the
driver directly. Those escape hatches are supported usage, not workarounds; see
[Direct Driver Access](advanced/driver-access.md).

## How this Guide is organized

* [Getting Started](getting-started.md) walks through the first model, client,
  save, and query.
* **Models** covers model kinds, generated builders, defaults, and validation.
* **Working with Data** covers persistence, typed queries, updates and
  deletion, aggregation, sessions and transactions, and bulk writes.
* **Indexes** covers declaring indexes, their establishment lifecycle, and
  drift detection with conservative reconciliation.
* **Advanced Usage** covers lifecycle hooks, direct driver access, and error
  handling with OxiMod's behavioral boundaries.
* **Reference** collects the attribute tables, API-choice guidance, and the
  runnable examples.

Precise API contracts live in the
[API documentation on docs.rs](https://docs.rs/oximod), and runnable workflows
live in the repository's
[`oximod/examples`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples)
directory.
