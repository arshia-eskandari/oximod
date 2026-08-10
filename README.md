# OxiMod

<p align="center">
  <strong>Schema-aware MongoDB modeling and typed queries for Rust</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/oximod"><img src="https://img.shields.io/crates/v/oximod" alt="Crates.io"></a>
  <a href="https://docs.rs/oximod"><img src="https://docs.rs/oximod/badge.svg" alt="Documentation"></a>
  <a href="https://crates.io/crates/oximod"><img src="https://img.shields.io/crates/d/oximod" alt="Downloads"></a>
  <a href="https://github.com/arshia-eskandari/oximod/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT License"></a>
</p>

---

## Overview

OxiMod is a schema-aware modeling layer built on top of the official MongoDB Rust driver. It adds model-oriented ergonomics—derive-generated construction, validation, defaults, indexes, lifecycle hooks, persistence helpers, and typed queries—without hiding MongoDB or restricting access to the driver.

OxiMod is best understood as:

> **MongoDB with stronger model ergonomics, not a replacement for the driver.**

Use OxiMod when you want concise, expressive model code and compile-time guidance for common MongoDB workflows, while retaining direct access to:

* `mongodb::Collection<Model>`;
* `mongodb::Collection<Document>`;
* raw BSON filters and updates;
* aggregation pipelines;
* sessions, compound indexes, and advanced driver options.

### Highlights

* One `Model` derive for collection-backed and embedded models
* Fluent generated builders with `Into<T>` setters
* Field defaults expressed as ordinary Rust expressions
* Aggregated built-in and custom validation
* Declarative single-field MongoDB indexes
* Global-client and explicit-client persistence workflows
* Type-aware filters, sorting, pagination, text search, and geospatial queries
* Typed single-document and bulk updates and deletions
* Typed nested paths for embedded documents and arrays of embedded models
* Optional lifecycle hooks for save and `_id` helper operations
* Structured validation and typed-query errors
* Full MongoDB driver escape hatches

---

## Installation

Add OxiMod and the dependencies used by your models and async runtime:

```bash
cargo add oximod mongodb
cargo add serde --features derive
cargo add tokio --features macros,rt-multi-thread
```

Add `async-trait` when implementing lifecycle hooks:

```bash
cargo add async-trait
```

A MongoDB server and connection URI are required for persistence and query execution.

---

## Quick start

Collection-backed models require a database name and collection name. Importing `oximod::Model` brings both the derive macro and the runtime persistence trait into scope; Rust keeps macro and type namespaces separate, so the shared name is intentional.

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

For complete runnable examples, see the [`oximod/examples`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples) directory.

---

## Model kinds

OxiMod uses the same derive for two distinct model kinds.

### Collection-backed models

Collection models are stored independently in MongoDB and require both:

```rust
#[db("database_name")]
#[collection("collection_name")]
```

They receive:

* generated `new()` and `Default` construction;
* fluent field setters;
* an inherent `validate()` method;
* field defaults and validation;
* lazy index initialization, with explicit `init_indexes()` startup initialization;
* optional lifecycle hooks;
* the `Model` persistence API;
* the `Queryable` typed-query API.

```rust
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,
    name: String,
    address: Address,
}
```

### Embedded models

Embedded models are values stored inside another model:

```rust
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
#[serde(rename_all = "camelCase")]
struct Address {
    street_name: String,
    city: String,
}
```

Embedded models receive:

* generated construction and setters;
* defaults;
* validation;
* typed nested-field metadata.

They do **not** receive:

* an independent MongoDB collection;
* `Model` persistence methods;
* root `Queryable` queries;
* indexes;
* lifecycle hooks.

Embedded values remain fully queryable through a collection model:

```rust
let users = User::query()
    .filter(|user| {
        user.address.nested(|address| {
            address.city.eq("City1")
        })
    })
    .all()
    .await?;
```

Generated field paths honor supported Serde `rename` and `rename_all` attributes, including nested paths.

---

## Generated builder API

Every derived model receives:

* `ModelType::new()`;
* `Default::default()`;
* a fluent setter for each field.

Ordinary setters accept values through `Into<T>`. Setters for `Option<T>` accept a value convertible into `T` and store it as `Some(...)`.

```rust
let user = User::new()
    .name("User1")
    .email("user1@example.com")
    .age(30)
    .active(true);
```

### Construction is not typestate

OxiMod does not require every setter to be called. During `new()`:

* fields with `#[default(...)]` use that expression;
* all other fields use `Default::default()`.

This means a required application field such as `String` begins as an empty string unless it has a configured default or is set through the builder. Use validation to enforce domain requirements.

### Defaults

`#[default(...)]` accepts an ordinary Rust expression convertible into the field type:

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct Preferences {
    #[default(String::from("en-CA"))]
    language: String,

    #[default(true)]
    notifications: bool,

    #[default(25_u32)]
    page_size: u32,

    nickname: Option<String>,
}
```

```rust
let preferences = Preferences::new().nickname("User1");

assert_eq!(preferences.language, "en-CA");
assert!(preferences.notifications);
assert_eq!(preferences.page_size, 25);
assert_eq!(preferences.nickname.as_deref(), Some("User1"));
```

Defaults are evaluated during construction and remain overridable through generated setters. Numeric literals should be suffixed when Rust's default literal type cannot convert into the field type.

Defaults are construction-time only; they are never applied when reading documents from MongoDB. A stored document that lacks the field entirely fails to deserialize regardless of `#[default(...)]`. When adding a field to a model whose collection already contains documents, give the field a read-side default with `#[serde(default = "path")]`. Avoid bare `#[serde(default)]` as a schema-evolution strategy: it substitutes the field type's `Default::default()` value — not your configured `#[default(...)]` — and a later save writes that substituted value back to the database.

### MongoDB `_id` setter

For a collection field named `_id`, the generated builder setter is `id()` by default:

```rust
let user = User::new()
    .id(ObjectId::new())
    .name("User1");
```

Rename it when needed:

```rust
#[document_id_setter_ident("with_id")]
```

```rust
let user = User::new()
    .with_id(ObjectId::new())
    .name("User1");
```

---

## Validation

Both collection and embedded models receive an inherent `validate()` method. OxiMod evaluates all configured rules and returns the failures together rather than stopping at the first invalid field.

```rust
let user = User::new()
    .email("not-an-email")
    .name("ab")
    .age(-1);

if let Err(error) = user.validate() {
    if let Some(errors) = error.validation_errors() {
        for failure in errors {
            println!("{}: {}", failure.field, failure.message);
        }
    }
}
```

Validation also runs automatically before:

* `save()`;
* `save_mut()`;
* `save_from()`;
* `save_from_mut()`.

For hook-enabled saves, the corresponding pre-save hook runs before validation, allowing mutable hooks to normalize or populate values before they are checked.

### Built-in validators

#### Optional values

| Validator  | Description                    |
| ---------- | ------------------------------ |
| `required` | Rejects `None` on `Option<T>`. |

Other validators on `Option<T>` run only when the option contains a value.

#### Length

| Validator        | Description                                                                |
| ---------------- | -------------------------------------------------------------------------- |
| `min_length = N` | Requires a length of at least `N`.                                         |
| `max_length = N` | Requires a length of at most `N`.                                          |
| `non_empty`      | Rejects empty collections and empty or whitespace-only string-like values. |

Length validation supports string-like values, arrays, sequential collections, sets, and maps where supported by the derive.

#### Strings

| Validator             | Description                           |
| --------------------- | ------------------------------------- |
| `email`               | Requires a valid basic email shape.   |
| `pattern = "..."`     | Matches a regular expression.         |
| `starts_with = "..."` | Requires the prefix.                  |
| `ends_with = "..."`   | Requires the suffix.                  |
| `includes = "..."`    | Requires the substring.               |
| `alphanumeric`        | Allows ASCII letters and digits only. |

String validation supports `String`, `str`-like values, and supported `Cow<str>` forms.

#### Numbers

| Validator       | Description                                     |
| --------------- | ----------------------------------------------- |
| `min = N`       | Inclusive minimum by default.                   |
| `max = N`       | Inclusive maximum by default.                   |
| `min_exclusive` | Changes `min` to a strict lower bound.          |
| `max_exclusive` | Changes `max` to a strict upper bound.          |
| `positive`      | Requires a value greater than zero.             |
| `negative`      | Requires a value less than zero.                |
| `non_negative`  | Requires a value greater than or equal to zero. |
| `non_positive`  | Requires a value less than or equal to zero.    |

#### Integers

| Validator         | Description                         |
| ----------------- | ----------------------------------- |
| `multiple_of = N` | Requires exact divisibility by `N`. |

### Custom validators

A custom validator is an ordinary function referenced by path:

```rust
fn validate_username(value: &String) -> Result<(), &'static str> {
    if value.eq_ignore_ascii_case("admin") {
        return Err("username is reserved");
    }

    Ok(())
}
```

```rust
#[validate(custom(crate::validate_username))]
username: String,
```

The function receives `&T` and may return any error implementing `ToString`. For `Option<T>`, it receives `&T` and runs only for `Some(value)`.

### Validation and embedded models

Validating a model evaluates only the rules declared on that model's own fields. `#[validate(...)]` rules declared inside an embedded model are **not** evaluated when a containing model is validated or saved: the parent can validate and save successfully while an embedded value violates its own rules. This applies to every container shape, including bare embedded fields, `Option<Embedded>`, and `Vec<Embedded>`. The embedded type's own `validate()` works normally when called directly.

To enforce embedded rules through the parent, add a custom validator on the containing field and delegate to the embedded value's `validate()`:

```rust
fn validate_address(address: &Address) -> Result<(), String> {
    address.validate().map_err(|error| error.to_string())
}
```

```rust
#[validate(custom(validate_address))]
address: Address,
```

A pre-save hook can serve as an alternative save-time guard. When hooks are used this way, implement **both** `pre_save` and `pre_save_mut` if the application uses both save forms: `save()`/`save_from()` run only `pre_save`, and `save_mut()`/`save_from_mut()` run only `pre_save_mut`, so a guard implemented in one hook does not protect the other save form.

### Validation and updates

Validation is **not** automatically applied to:

* raw MongoDB update documents;
* typed `update_one()` or `update_all()` expressions;
* direct collection operations.

These operations modify stored documents through MongoDB. Application code remains responsible for choosing values that preserve model invariants.

---

## Typed queries

Import `Queryable` to call `ModelType::query()`:

```rust
use oximod::Queryable;
```

The derive generates a typed field structure for each collection model. Every field exposes only operations supported by its Rust type, so incompatible code fails to compile. For example:

* regex methods are unavailable on integer fields;
* ordered comparisons are unavailable on booleans;
* `unset()` is unavailable on required fields;
* array operations are unavailable on scalar fields;
* nested operations require embedded-model field metadata.

Typed queries currently execute through the global `OxiClient`.

### Filters and logical expressions

```rust
let users = User::query()
    .filter(|user| {
        user.active.eq(true)
            & user.age.gte(18)
            & (
                user.name.eq("User1")
                    | user.name.eq("User2")
            )
    })
    .all()
    .await?;
```

Use:

* `&` for logical AND;
* `|` for logical OR;
* `not(...)` for field-level negation.

Rust does not allow overloading `&&` or `||`. Repeated `filter()` calls are also combined with AND:

```rust
let users = User::query()
    .filter(|user| user.active.eq(true))
    .filter(|user| user.age.gte(18))
    .all()
    .await?;
```

### Query-operation families

Depending on the field type, generated fields support families such as:

* equality: `eq`, `ne`;
* membership: `in_values`, `not_in_values`;
* ordered comparisons: `gt`, `gte`, `lt`, `lte`;
* field presence: `exists`, `not_exists`;
* optional null checks such as `is_null`;
* BSON type checks;
* regex and escaped string helpers;
* modulo and integer bitwise predicates;
* arrays and `$elemMatch`;
* embedded-model paths;
* GeoJSON geospatial predicates.

Ordered comparisons, numeric updates, and modulo are also available on `Option<T>` fields whose inner type supports them. The operand is always a value of the inner type `T` — for example `expires_at.gt(deadline)` on an `Option<DateTime>` field, or `login_count.inc(1)` on an `Option<i32>` field; `Some(...)` and `None` operands do not compile. Documents storing BSON null and documents missing the field follow MongoDB's normal query semantics: an ordered comparison against an inner value does not match them.

The Rust field name does not have to match the stored MongoDB field name. Generated paths follow supported Serde renames:

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("app")]
#[collection("work_items")]
struct WorkItem {
    team_name: String,
}
```

```rust
WorkItem::query()
    .filter(|item| item.team_name.eq("Team1"))
    .all()
    .await?;
```

The generated query targets `teamName` in MongoDB.

`#[serde(alias = "...")]` is a read-side compatibility tool, not a rename migration. Typed query paths always use a field's primary serialized name, so documents still stored under a legacy key remain readable through the alias but are silently missed by typed filters on that field. During a field rename, migrate the persisted documents — or match both spellings with a raw `$or` filter through the document collection — before relying on typed queries against the renamed field.

### Sorting

```rust
let users = User::query()
    .sort_by(|user| user.age.desc())
    .then_sort_by(|user| user.name.asc())
    .all()
    .await?;
```

* `sort_by()` establishes or replaces the primary sort;
* `then_sort_by()` appends another sort field.

Use deterministic secondary sorting when several documents may share the same primary value.

### Limits, skipping, and pagination

```rust
let page = User::query()
    .filter(|user| user.active.eq(true))
    .sort_by(|user| user.name.asc())
    .page(2, 25)
    .all()
    .await?;
```

Pagination is one-based. Page `2` with size `25` skips the first `25` matching documents.

Invalid pagination values and limits that cannot be represented by the MongoDB driver are returned as typed query errors when the query executes.

A page is read as one result window through `all()`, which fails as a whole if any document in the window cannot be deserialized into the model — documents are never silently dropped, and none of the window's documents are returned. Later pages whose windows contain only valid documents still succeed. To locate, inspect, or repair documents that no longer match the model, read them as raw BSON through `get_document_collection()`.

### Read execution semantics

| Method    | Result          | Filter | Sort | Skip | Limit / page |
| --------- | --------------- | -----: | ---: | ---: | -----------: |
| `all()`   | `Vec<Model>`    |    Yes |  Yes |  Yes |          Yes |
| `first()` | `Option<Model>` |    Yes |  Yes |   No |           No |
| `count()` | `u64`           |    Yes |   No |   No |           No |

`count()` uses the filter and configured text search, but ignores result-ordering and result-window modifiers.

### Arrays

Array fields support typed membership and update operations:

```rust
let users = User::query()
    .filter(|user| {
        user.tags.contains_all(["rust", "mongodb"])
            & user.scores.elem_match(|score| {
                score.gte(60) & score.lte(100)
            })
    })
    .all()
    .await?;
```

Query helpers include:

* element membership;
* `$all`;
* exact `$size`;
* scalar `$elemMatch`.

The scalar `elem_match` overload applies to scalar elements; for arrays of embedded models use `elem_match_nested` (see [Embedded documents](#embedded-documents)).

Typed array updates include:

* `$push` and multi-value `$push`;
* `$addToSet` and multi-value `$addToSet`;
* `$pull`;
* first- and last-element `$pop`;
* positional and filtered updates for arrays of embedded models.

Array update operators (`push`, `add_to_set`, `pull`, and whole-array `set`) require the element type to convert into BSON (`Into<Bson>`). Scalar elements such as strings and numbers qualify automatically; derived embedded models do not implement `Into<Bson>` automatically. Implement the conversion once per embedded type to enable these operators on `Vec<Embedded>` fields:

```rust
use mongodb::bson::{Bson, to_bson};

impl From<Address> for Bson {
    fn from(address: Address) -> Self {
        to_bson(&address).expect("Address serializes to BSON")
    }
}
```

`From` conversions cannot report failure, so the implementation must decide how to handle a value that fails to serialize (this example panics). Typed *matching* on embedded arrays needs no conversion — `elem_match_nested` works without it.

### Embedded documents

```rust
let users = User::query()
    .filter(|user| {
        user.address.nested(|address| {
            address.city.eq("City1")
        })
    })
    .all()
    .await?;
```

Optional embedded models support the same nested field schema when present. Arrays of embedded models add typed nested `$elemMatch`:

```rust
let users = User::query()
    .filter(|user| {
        user.addresses.elem_match_nested(|address| {
            address.city.eq("City1")
                & address.active.eq(true)
        })
    })
    .all()
    .await?;
```

An `elem_match_nested` filter also supplies the array match that MongoDB's positional `$` update operator requires, so pair it with `positional(...)` when updating the first matched element.

Nested fields may also be used for sorting and typed updates where supported.

### String and regex queries

String fields support regular expressions and escaped convenience helpers:

```rust
use oximod::RegexOption;

let users = User::query()
    .filter(|user| {
        user.name.matches_regex_with_options(
            "^user",
            [RegexOption::CaseInsensitive],
        )
    })
    .all()
    .await?;
```

`RegexOption` maps to MongoDB's common regex options:

* case-insensitive;
* multiline;
* dot matches newline;
* ignore pattern whitespace.

Convenience helpers such as prefix, suffix, and contained-text checks escape their input before constructing the regex.

### Text search

Text search requires an appropriate MongoDB text index:

```rust
#[index(text)]
content: String,
```

Use a string for a basic search:

```rust
let articles = Article::query()
    .text("rust mongodb")
    .all()
    .await?;
```

Use `TextSearch` for additional options:

```rust
use oximod::TextSearch;

let articles = Article::query()
    .text(
        TextSearch::new("\"rust mongodb\" -beginner")
            .language("none")
            .case_sensitive(false)
            .diacritic_sensitive(false),
    )
    .sort_by_text_score()
    .all()
    .await?;
```

MongoDB phrase and excluded-term syntax can be included in the search string.

### Geospatial queries

OxiMod provides typed GeoJSON values:

* `GeoPoint`;
* `GeoPolygon`;
* `NearQuery`.

```rust
use oximod::{GeoPoint, NearQuery};

let places = Place::query()
    .filter(|place| {
        place.location.near(
            NearQuery::new(
                GeoPoint::new(-79.38, 43.65),
            )
            .max_distance(5_000.0),
        )
    })
    .all()
    .await?;
```

GeoJSON coordinates use longitude-latitude order. With a `2dsphere` index, GeoJSON `$near` distances are expressed in metres.

Typed geospatial predicates include `$near`, `$geoWithin`, and `$geoIntersects` where supported by the field geometry.

OxiMod serializes the geometry but does not fully validate coordinate ranges, polygon validity, or distance relationships; MongoDB remains the final authority for the query.

---

## Typed updates

Typed update expressions are built from the same generated fields used for filters:

```rust
let updated = User::query()
    .filter(|user| user.email.eq("user1@example.com"))
    .update_one(|user| {
        user.active.set(true)
            & user.age.inc(1)
            & user.nickname.unset()
            & user.tags.add_to_set("verified")
    })
    .await?;
```

Combine independent update expressions with `&`. Updates using the same MongoDB operator are merged into one operator document.

Typed update expressions write specific dotted field paths. When documents written by older model versions may still be present, prefer these targeted updates — or `$set` on dotted paths through `update_by_id` — over replacing a whole stored document with a serialized model: a whole-document replacement writes only the current struct shape and can drop or rewrite fields the running code no longer declares.

Supported families include:

* scalar `$set`;
* optional-field `$unset`;
* numeric `$inc`, `$mul`, `$min`, and `$max`;
* field rename and current-date updates;
* array push, set-like addition, pull, and pop operations;
* positional and array-filtered updates for embedded arrays.

Numeric updates on `Option<T>` fields take inner-type operands, exactly as on required fields.

### Single-document update

```rust
let updated = User::query()
    .filter(|user| user.active.eq(false))
    .sort_by(|user| user.age.asc())
    .update_one(|user| user.active.set(true))
    .await?;
```

`update_one()`:

* applies the filter;
* applies sorting to choose one match;
* returns the document after the update as `Option<Model>`;
* ignores skip, limit, and pagination.

### Bulk update

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .update_all(|user| user.active.set(true))
    .await?;
```

`update_all()` returns MongoDB's `UpdateResult` and rejects sorting, skipping, limiting, and pagination instead of silently ignoring them.

> **Warning:** An unfiltered `update_all()` affects every document in the collection.

---

## Typed deletion

### Single-document deletion

```rust
let deleted = User::query()
    .filter(|user| user.active.eq(false))
    .sort_by(|user| user.age.asc())
    .delete_one()
    .await?;
```

`delete_one()`:

* applies the filter;
* applies sorting to choose one match;
* returns the deleted document as `Option<Model>`;
* ignores skip, limit, and pagination.

### Bulk deletion

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .delete_all()
    .await?;
```

`delete_all()` returns MongoDB's `DeleteResult` and rejects sorting, skipping, limiting, and pagination.

> **Warning:** An unfiltered `delete_all()` affects every document in the collection.

---

## Model persistence API

The `Model` trait is implemented only for collection-backed models.

### Global-client methods

| Method                      | Description                                                                                               |
| --------------------------- | --------------------------------------------------------------------------------------------------------- |
| `save()`                    | Validates and inserts `&self`, returning the inserted `ObjectId`. Runs immutable save hooks when enabled. |
| `save_mut()`                | Runs mutable pre-save logic, validates, inserts, and then runs the mutable post-save hook.                |
| `find_by_id(id)`            | Returns `Option<Model>` for a MongoDB `ObjectId`.                                                         |
| `update_by_id(id, update)`  | Applies a raw MongoDB update document and returns `UpdateResult`.                                         |
| `delete_by_id(id)`          | Deletes by `ObjectId` and returns `DeleteResult`.                                                         |
| `exists(filter)`            | Returns whether at least one raw BSON filter match exists.                                                |
| `count(filter)`             | Counts documents matching a raw BSON filter.                                                              |
| `clear()`                   | Deletes every document in the model's collection.                                                         |
| `get_collection()`          | Returns `mongodb::Collection<Model>`.                                                                     |
| `get_document_collection()` | Returns `mongodb::Collection<Document>`.                                                                  |

> **Warning:** `clear()` removes the entire collection contents.

### Explicit-client counterparts

Every persistence or collection-access operation has an explicit-client counterpart:

* `save_from()`;
* `save_from_mut()`;
* `find_by_id_from()`;
* `update_by_id_from()`;
* `delete_by_id_from()`;
* `exists_from()`;
* `count_from()`;
* `clear_from()`;
* `get_collection_from()`;
* `get_document_collection_from()`.

These methods accept `&mongodb::Client` and do not require the global client.

---

## Client management

OxiMod supports two independent client patterns.

### Global client

Initialize the process-wide client once during application startup:

```rust
OxiClient::init_global(
    "mongodb://localhost:27017".to_string(),
)
.await?;
```

Methods without an `_from` suffix and all typed-query execution methods use this client.

```rust
let user_id = user.save().await?;
let users = User::query().all().await?;
```

`OxiClient::global()` returns an `Arc<mongodb::Client>` and fails when global initialization has not completed. A second successful initialization is not allowed.

Treat `init_global()` as a process-level, one-time startup step and handle its `Result`. Global-client operations require initialization to have completed successfully; until it has, they fail. Once a client is installed, later `init_global()` calls return an error rather than replacing it.

### Instance-level client

```rust
let owner = OxiClient::new(
    "mongodb://localhost:27017".to_string(),
)
.await?;

let client = owner
    .client()
    .expect("OxiClient::new initializes its client");

let user_id = user.save_from(client).await?;
```

Instance-level clients are useful for:

* dependency injection;
* integration tests;
* multi-tenant systems;
* multiple MongoDB deployments;
* code that deliberately avoids global state.

`OxiClient::default()` starts without an inner client. Initialize it later with `init_client()`.

### `OxiClient` API

| Method             | Description                                          |
| ------------------ | ---------------------------------------------------- |
| `new(uri)`         | Creates an initialized instance-level wrapper.       |
| `init_client(uri)` | Initializes or replaces the wrapper's client.        |
| `client()`         | Returns `Option<&mongodb::Client>`.                  |
| `client_mut()`     | Returns `Option<&mut mongodb::Client>`.              |
| `init_global(uri)` | Initializes the process-wide client once.            |
| `global()`         | Returns the shared client as `Arc<mongodb::Client>`. |

### Typed-query limitation

Typed query builders currently execute only through the global client. There is no explicit-client typed-query executor yet. In an explicit-client workflow, use:

* the `_from` model helpers;
* `get_collection_from()`;
* `get_document_collection_from()`.

---

## Direct MongoDB collection access

OxiMod intentionally preserves access to the official driver.

### Typed collection

```rust
let collection = User::get_collection()?;
```

Returns:

```rust
mongodb::Collection<User>
```

Use it for:

* raw BSON queries with typed deserialization;
* aggregation;
* driver-specific options;
* sessions;
* operations not represented by OxiMod's helpers.

### Raw document collection

```rust
let collection = User::get_document_collection()?;
```

Returns:

```rust
mongodb::Collection<mongodb::bson::Document>
```

Use it when the document shape is dynamic or when working directly with BSON.

### Sessions and transactions

Sessions and transactions are raw-driver territory: no OxiMod method accepts a `ClientSession`. A model or typed-query write issued while a transaction is open executes outside that transaction and commits independently, with no error or warning. Once any write to a collection participates in a transaction, perform **every** write to that collection through the session-aware driver APIs on `get_collection()` / `get_document_collection()` for as long as the transactional pattern is in use.

### Aggregation example

```rust
use futures_util::TryStreamExt;
use mongodb::bson::doc;

let collection = User::get_collection()?;

let pipeline = vec![
    doc! {
        "$match": {
            "active": true,
        },
    },
    doc! {
        "$group": {
            "_id": "$role",
            "count": {
                "$sum": 1,
            },
        },
    },
];

let results = collection
    .aggregate(pipeline)
    .await?
    .try_collect::<Vec<_>>()
    .await?;
```

Raw filters and pipelines must use serialized MongoDB field names. Unlike typed queries, the compiler cannot verify those paths or operator compatibility.

---

## Indexes

Declare a single-field MongoDB index directly on a collection-model field:

```rust
#[index(unique, name = "email_idx")]
email: String,
```

Declared indexes are not created by deriving the model. Generated indexes are initialized lazily before model insertion. A successful initialization is remembered for that model type within the process. Merely constructing a query or obtaining a collection does not create indexes.

Applications that need declared indexes before their first write — for example so a unique constraint is enforced from process start — can establish them explicitly during startup:

```rust
User::init_indexes().await?;              // global client
User::init_indexes_from(&client).await?;  // explicit client
```

Explicit initialization reuses the save path's index machinery and shares its once-per-process establishment state: repeated successful calls are harmless, an index-establishment failure returns the same `OxiModError::Index` surface and can be retried by a later call or save, and applications that never call these methods keep the existing lazy save-triggered behavior. This is establishment, not drift synchronization: an index dropped or changed externally after a successful initialization is not automatically re-established during the same process.

Use direct collection access for compound indexes and advanced options not represented by `#[index(...)]`.

Partial or filtered indexes (MongoDB's `partialFilterExpression` option) are likewise not expressible with `#[index(...)]`; create them through the driver's `create_index` on the collection returned by `get_collection()` or `get_document_collection()`. Driver-created indexes coexist with `#[index(...)]` declarations. MongoDB enforces that uniqueness; the underlying driver failure on a violation is a duplicate-key error (E11000), not an OxiMod validation failure — OxiMod validation does not replace MongoDB's index enforcement.

> **Warning:** Do not emulate a compound unique index by storing a derived composite-key field guarded by `#[index(unique)]`. Partial updates such as `update_by_id` or a typed `$set` can change the source fields without recomputing the derived field, silently desynchronizing it; genuine duplicates can then persist while the index still appears healthy. Create a real MongoDB compound unique index through the driver instead.

### Core options

| Option                     | Description                                      |
| -------------------------- | ------------------------------------------------ |
| `unique`                   | Enforces unique values.                          |
| `sparse`                   | Excludes documents where the field is missing.   |
| `hidden`                   | Hides the index from the query planner.          |
| `name = "..."`             | Assigns an explicit index name.                  |
| `order = 1` / `order = -1` | Creates an ascending or descending scalar index. |
| `expire_after_secs = N`    | Creates a TTL index.                             |
| `background`               | Forwards MongoDB's background option.            |

### Specialized index types

| Option         | Description                     |
| -------------- | ------------------------------- |
| `text`         | Creates a text index.           |
| `hashed`       | Creates a hashed index.         |
| `wildcard`     | Creates a wildcard field index. |
| `geo_2dsphere` | Creates a `2dsphere` index.     |
| `geo_2d`       | Creates a planar `2d` index.    |

### Advanced options

| Option                           | Description                                                   |
| -------------------------------- | ------------------------------------------------------------- |
| `version = N`                    | Selects a standard index version or custom version value.     |
| `text_index_version = N`         | Selects the text-index version.                               |
| `geo_2dsphere_index_version = N` | Selects the `2dsphere` index version.                         |
| `weight = N`                     | Sets the field's text-index weight.                           |
| `default_language = "..."`       | Sets the text index's default language.                       |
| `language_override = "..."`      | Sets the document field used to override language.            |
| `case_insensitive`               | Applies OxiMod's English secondary-strength collation preset. |
| `bits = N`                       | Sets `2d` precision.                                          |
| `min = N`                        | Sets the lower `2d` coordinate bound.                         |
| `max = N`                        | Sets the upper `2d` coordinate bound.                         |

Text-specific options such as `weight`, `default_language`, `language_override`, and `text_index_version` imply a text index.

### Index examples

```rust
#[index(unique, sparse, name = "email_idx")]
email: Option<String>,

#[index(text, weight = 10, default_language = "english")]
title: String,

#[index(geo_2dsphere)]
location: GeoPoint,

#[index(expire_after_secs = 3600)]
expires_at: mongodb::bson::DateTime,
```

### Index-initialization attributes

The derive currently accepts:

```rust
#[index_max_retries(N)]
#[index_max_init_seconds(N)]
```

These values are stored in the generated index coordinator, but the current runtime does not enforce them as hard retry or timeout limits. Do not rely on them as operational guarantees yet.

---

## Lifecycle hooks

Lifecycle hooks are optional. Enable them on a collection model with `#[hooks]`, then implement `Hooks`:

```rust
use oximod::{Hooks, Model, OxiModError};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
#[hooks]
struct User {
    email: String,
    name: String,
}

#[async_trait::async_trait]
impl Hooks for User {
    async fn pre_save_mut(
        &mut self,
    ) -> Result<(), OxiModError> {
        self.email = self.email.trim().to_lowercase();
        self.name = self.name.trim().to_string();
        Ok(())
    }
}
```

Every hook has a default no-op implementation. Override only the events the model needs.

### Save hooks

| Hook            | Runs for                    | Behavior                                                                             |
| --------------- | --------------------------- | ------------------------------------------------------------------------------------ |
| `pre_save`      | `save`, `save_from`         | Immutable check before validation and insertion.                                     |
| `post_save`     | `save`, `save_from`         | Runs after insertion.                                                                |
| `pre_save_mut`  | `save_mut`, `save_from_mut` | May mutate the model before validation and insertion.                                |
| `post_save_mut` | `save_mut`, `save_from_mut` | May mutate in-memory state after insertion; changes are not automatically persisted. |

### `_id` helper hooks

| Hook                         | Runs for                            |
| ---------------------------- | ----------------------------------- |
| `pre_find` / `post_find`     | `find_by_id`, `find_by_id_from`     |
| `pre_update` / `post_update` | `update_by_id`, `update_by_id_from` |
| `pre_delete` / `post_delete` | `delete_by_id`, `delete_by_id_from` |

### Hook boundaries

Hooks do **not** wrap:

* typed-query reads, updates, or deletions;
* direct typed or raw collection operations;
* `clear`;
* `exists`;
* `count`;
* collection accessors.

A pre-hook error prevents the associated database operation. A post-hook error is returned after the database operation has already succeeded.

---

## Attributes reference

### Struct-level attributes

| Attribute                             | Applies to        | Description                                                        |
| ------------------------------------- | ----------------- | ------------------------------------------------------------------ |
| `#[model(embedded)]`                  | Embedded models   | Marks a model as embedded instead of collection-backed.            |
| `#[db("name")]`                       | Collection models | Required database name.                                            |
| `#[collection("name")]`               | Collection models | Required collection name.                                          |
| `#[document_id_setter_ident("name")]` | Collection models | Renames the generated `_id` setter.                                |
| `#[hooks]`                            | Collection models | Generates lifecycle-hook calls.                                    |
| `#[index_max_retries(N)]`             | Collection models | Accepted and stored; not currently enforced as a hard retry limit. |
| `#[index_max_init_seconds(N)]`        | Collection models | Accepted and stored; not currently enforced as a hard timeout.     |

### Field-level attributes

| Attribute                | Description                                                          |
| ------------------------ | -------------------------------------------------------------------- |
| `#[default(expression)]` | Replaces the field's `Default::default()` initialization expression. |
| `#[validate(...)]`       | Adds built-in or custom validation rules.                            |
| `#[index(...)]`          | Adds a generated single-field MongoDB index on a collection model.   |

Serde field and container renames are used when generating typed query paths.

---

## Error handling

Most OxiMod operations return `OxiModError`. Its variants distinguish failures involving:

* MongoDB client construction;
* missing or duplicate global-client initialization;
* serialization and deserialization;
* aggregation;
* index initialization;
* model validation;
* database operations;
* user-defined custom errors;
* typed-query configuration.

Driver-backed variants retain their source errors.

### Validation errors

```rust
if let Err(error) = user.validate() {
    if let Some(errors) = error.validation_errors() {
        for error in errors {
            println!("{}: {}", error.field, error.message);
        }
    }
}
```

Each `ValidationError` exposes:

* `field`;
* `message`.

Several rules may produce several messages for the same field.

### Query errors

Typed-query configuration failures are exposed through `OxiModError::Query` and `query_error()`. They include:

* zero page numbers;
* zero page sizes;
* pagination overflow;
* limits outside the driver's supported integer range;
* unsupported sort, skip, limit, or pagination modifiers on bulk writes.

```rust
use oximod::{OxiModError, QueryError};

match User::query().page(0, 20).all().await {
    Err(OxiModError::Query(
        QueryError::InvalidPageNumber { page },
    )) => {
        println!("Invalid page number: {page}");
    }
    Err(error) => return Err(error.into()),
    Ok(users) => println!("Found {} users", users.len()),
}
```

---

## Choosing the right API

| Goal                                               | Recommended API                      |
| -------------------------------------------------- | ------------------------------------ |
| Construct and validate a model                     | Generated builder and `validate()`   |
| Save or work by `_id`                              | `Model` methods                      |
| Type-safe filters, sorting, pagination, and writes | `Queryable`                          |
| Explicit-client persistence                        | `_from` model methods                |
| Raw filters with typed model results               | `Collection<Model>`                  |
| Dynamic BSON documents                             | `Collection<Document>`               |
| Aggregation pipelines                              | Direct MongoDB collection access     |
| Compound, partial/filtered, or unsupported index options | MongoDB driver index API       |
| Sessions and advanced driver features              | Direct MongoDB collection/client API |

OxiMod is designed so these approaches can coexist in the same application.

---

## Examples

The repository includes focused runnable examples covering:

* aggregation;
* basic persistence;
* `_id` workflows;
* custom validation;
* defaults;
* typed deletion;
* lifecycle hooks;
* raw MongoDB queries;
* typed queries;
* typed updates;
* explicit-client workflows;
* structured validation errors;
* built-in validation.

Browse them in [`oximod/examples`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples).

Run an example with:

```bash
cargo run -p oximod --example typed_query
```

MongoDB-backed examples read `MONGODB_URI` from the environment or a `.env` file.

---

## Current behavioral notes

* Typed-query execution currently requires the global client.
* Typed and raw update operations do not automatically run model validation.
* Validation does not descend into embedded models; enforce embedded rules with a custom validator on the containing field or with pre-save hooks (covering both `pre_save` and `pre_save_mut`).
* Generated indexes are single-field and are initialized lazily during saves, or explicitly at startup with `init_indexes()` / `init_indexes_from(&client)`; initialization is once per process and does not re-establish indexes dropped externally afterward.
* Compound and partial/filtered indexes require the MongoDB driver API; a derived composite-key field with `#[index(unique)]` is not a safe substitute for a compound unique index.
* OxiMod methods do not accept MongoDB sessions; writes issued through OxiMod while a transaction is open commit outside that transaction.
* Typed reads fail as a whole when any document in the selected result window cannot be deserialized; use the raw document collection to inspect or repair such documents.
* Lifecycle hooks wrap only save and `_id` helper methods.
* `clear()`, unfiltered `update_all()`, and unfiltered `delete_all()` can affect an entire collection.
* `GeoPoint`, `GeoPolygon`, and `NearQuery` construct MongoDB geometry and query documents but do not perform complete geospatial validity checks.
* `index_max_retries` and `index_max_init_seconds` are accepted but are not currently enforced as hard limits.

---

## Documentation

* [API documentation on docs.rs](https://docs.rs/oximod)
* [Repository](https://github.com/arshia-eskandari/oximod)
* [Runnable examples](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples)
* [Crate on crates.io](https://crates.io/crates/oximod)

---

## License

OxiMod is licensed under the [MIT License](https://github.com/arshia-eskandari/oximod/blob/main/LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in OxiMod shall be licensed under the MIT License without additional terms or conditions.

