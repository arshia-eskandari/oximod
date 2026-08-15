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
* raw aggregation pipelines;
* sessions, compound indexes, and advanced driver options.

### Highlights

* One `Model` derive for collection-backed and embedded models
* Fluent generated builders with `Into<T>` setters
* Field defaults expressed as ordinary Rust expressions
* Aggregated built-in and custom validation
* Declarative single-field MongoDB indexes
* Explicit index drift inspection with conservative create-only reconciliation
* Global-client and explicit-client persistence workflows
* Type-aware filters, sorting, pagination, text search, and geospatial queries
* Typed single-document and multi-document updates and deletions
* Typed model-scoped bulk writes batching mixed operations into one driver bulk-write action
* First-class aggregation builder mixing typed stages, raw stages, and typed output
* Typed nested paths for embedded documents and arrays of embedded models
* Explicit session-aware operations for MongoDB transactions
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
* explicit index drift inspection and create-only reconciliation;
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
* `save_from_mut()`;
* `save_with_session()`;
* `save_mut_with_session()`;
* bulk-write `insert`, `insert_many`, and `replace_one` execution (as the whole-model preflight).

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

#### Nested embedded models

| Validator | Description                                                             |
| --------- | ----------------------------------------------------------------------- |
| `nested`  | Recursively validates embedded OxiMod models reached through the field. |

`nested` is valid only on fields whose type resolves through the supported containers to a `#[model(embedded)]` model; other target types are rejected at compile time. See [Validation and embedded models](#validation-and-embedded-models).

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

Validation descends into an embedded model only where the containing field explicitly opts in with `#[validate(nested)]`. A field without the attribute keeps the previous behavior: the parent validates and saves without evaluating the embedded value's own rules, and the embedded type's `validate()` works normally when called directly.

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,

    #[validate(pattern = r"^[0-9]{5}$")]
    postal_code: String,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(nested)]
    address: Address,

    #[validate(nested)]
    previous_addresses: Vec<Address>,
}
```

One `nested` marker descends recursively through supported container wrappers until it reaches the embedded model:

* a bare embedded field;
* `Option<Embedded>`;
* `Vec<Embedded>`;
* `HashMap<String, Embedded>`;
* recursive combinations such as `Vec<Option<Embedded>>`, `Option<Vec<Embedded>>`, and `HashMap<String, Vec<Option<Embedded>>>`.

Each model-to-model containment edge remains opt-in: when an embedded model contains another embedded model, the inner containing field must also carry `#[validate(nested)]` for validation to descend further. Marking a field whose type does not resolve to an embedded model (such as a scalar or a collection-backed model) fails at compile time.

Container semantics compose with the existing rules:

* `None` produces no nested errors; add `required` to reject absence;
* empty vectors and maps produce no nested errors; add `non_empty` to reject emptiness;
* the field's own rules and its descendants' rules are all evaluated and aggregated together with the rest of the model's failures.

Descendant failures keep their exact messages and report path-aware `field` values:

```text
address.postal_code
previous_addresses[1].city
addresses["billing"].postal_code
orders[2].shipping_address.postal_code
```

Paths use Rust model field names, matching top-level validation attribution; Serde/BSON renames affect stored documents and typed queries, not validation paths. Map keys are quoted with `"` and `\` escaped, so keys containing dots, spaces, brackets, or quotes cannot masquerade as path segments. Error order is deterministic: parent declaration order, depth-first into each nested model's own declaration order, vector elements in ascending index order, and map entries in lexicographically sorted key order. The path strings are deterministic and human-readable; they are not a stable machine-parseable schema.

Because `#[validate(nested)]` changes what the model's own validation evaluates, it applies automatically anywhere whole-model validation already runs: `validate()`, every save form (including session-aware saves), and the bulk-write insert/replace preflight. Typed and raw update expressions remain non-validating (see [Validation and updates](#validation-and-updates)).

The previous custom-validator delegation pattern remains valid — for example to combine descent with cross-field checks — but is no longer required merely to descend into an embedded value:

```rust
fn validate_address(address: &Address) -> Result<(), String> {
    address.validate().map_err(|error| error.to_string())
}
```

```rust
#[validate(custom(validate_address))]
address: Address,
```

Custom delegation reports the child's failures as one error attributed to the containing field, while `nested` reports each descendant failure under its own path. Pre-save hook guards likewise remain possible but are no longer required for descent; lifecycle hooks are unchanged by nested validation.

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

### Multi-document update

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .update_all(|user| user.active.set(true))
    .await?;
```

`update_all()` updates every matching document in one command, returns MongoDB's `UpdateResult`, and rejects sorting, skipping, limiting, and pagination instead of silently ignoring them.

To batch several independent write intentions — including inserts and deletes — into one OxiMod bulk execution, see [Bulk writes](#bulk-writes).

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

### Multi-document deletion

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .delete_all()
    .await?;
```

`delete_all()` deletes every matching document in one command, returns MongoDB's `DeleteResult`, and rejects sorting, skipping, limiting, and pagination.

> **Warning:** An unfiltered `delete_all()` affects every document in the collection.

---

## Bulk writes

`ModelType::bulk_write()` (a `Model` method) queues multiple independent write intentions against one model's collection and submits them to MongoDB as **one driver bulk-write action** — never by expanding the batch into per-item OxiMod save/update/delete calls. (The MongoDB driver may split a sufficiently large logical batch into multiple server commands to satisfy server/message batch limits; that splitting belongs to the driver.) The batch may mix all six operation kinds freely, and OxiMod preserves the queue order exactly:

```rust
use oximod::{Model, Queryable};

Job::init_indexes().await?; // establish the unique dedupe index first

let result = Job::bulk_write()
    .insert(Job::new().dedupe_key("job-1").status("queued"))
    .insert_many(more_jobs)
    .update_one(
        Job::query().filter(|job| job.dedupe_key.eq("job-3")),
        |job| job.status.set("ready"),
    )
    .update_many(
        Job::query().filter(|job| job.status.eq("stale")),
        |job| job.status.set("expired"),
    )
    .replace_one(
        Job::query().filter(|job| job.dedupe_key.eq("job-4")),
        replacement_job,
    )
    .delete_one(Job::query().filter(|job| job.dedupe_key.eq("job-5")))
    .delete_many(Job::query().filter(|job| job.status.eq("expired")))
    .ordered(false)
    .execute()
    .await?;

println!("inserted {}", result.inserted_count);
```

Bulk writes require **MongoDB Server 8.0+**. OxiMod does not emulate the command on older servers: a server that cannot execute it fails through the ordinary bulk-write error path, and OxiMod never silently expands a batch into individual writes. Retryable-write behavior belongs to the MongoDB driver; OxiMod adds no retry loop.

### Typed construction

Update and delete operations consume ordinary `Query<M>` values, so filters, text search, and array filters reuse the exact typed construction and Serde-renamed paths of the rest of the query API, and updates reuse `UpdateExpression`. Modifiers a driver bulk model cannot represent are rejected locally — before any network communication — as `QueryError::UnsupportedBulkWriteModifier`, never silently dropped:

* `update_one` and `replace_one` support a query sort (it selects which matching document is written); skip, limit, and pagination are rejected;
* `update_many` supports array filters; sort, skip, limit, and pagination are rejected;
* `delete_one` and `delete_many` cannot carry a sort (unlike `Query::delete_one`), and reject skip, limit, pagination, and array filters.

### Validation, hooks, and indexes

Whole model values queued through `insert`, `insert_many`, and `replace_one` run `#[validate]` for **every** queued value before any network communication: if the 101st queued insert is invalid, the batch returns `OxiModError::Validation` and zero writes are sent. Typed update expressions remain non-validating, exactly like `update_one()`/`update_all()`.

Bulk-write operations are a separate execution surface. They preserve typed query/update construction and whole-model validation where applicable, but they do **not** invoke OxiMod lifecycle hooks.

Executing a bulk write never establishes declared `#[index(...)]` specifications. Initialize indexes explicitly — especially unique constraints used as idempotency guards — with `init_indexes()` / `init_indexes_from(&client)` before bulk ingest.

Replacements follow ordinary MongoDB semantics: `replace_one` rewrites the **whole stored document** with the serialized model, removing fields the Rust model does not declare. Prefer targeted typed updates when documents written by other application versions may exist.

### Ordered and unordered execution

MongoDB executes bulk writes in order by default and stops after the first failing operation. With `.ordered(false)`, the server continues attempting the remaining operations after a failure and may reorder execution for performance; do not depend on execution order when unordered. `.with_options(BulkWriteOptions)` passes the driver's full options through (ordered execution, server-side `bypass_document_validation`, comment, `let` variables, write concern); `bypass_document_validation` never disables OxiMod's client-side `#[validate]` preflight.

### Execution and results

Summary terminals return the driver's `SummaryBulkWriteResult` (inserted, matched, modified, upserted, and deleted counts); `_verbose` terminals return `VerboseBulkWriteResult` with per-operation results keyed by each operation's original queued index:

* `execute()` / `execute_verbose()` — global client;
* `execute_from(&client)` / `execute_verbose_from(&client)` — explicit client;
* `execute_with_session(&mut session)` / `execute_verbose_with_session(&mut session)` — session/transaction execution through the session's own client.

### Partial failures keep their operation indexes

When individual writes fail, the returned `OxiModError::BulkWrite` preserves the driver's `mongodb::error::BulkWriteError`: `write_errors` maps each failure back to its **original queued operation index**, and `partial_result` describes the writes that succeeded. `OxiModError::bulk_write_error()` reaches that detail without manual downcasting:

```rust
match Job::bulk_write().insert_many(jobs).ordered(false).execute().await {
    Ok(result) => println!("inserted {}", result.inserted_count),
    Err(error) => {
        if let Some(failure) = error.bulk_write_error() {
            for (index, write_error) in &failure.write_errors {
                eprintln!(
                    "operation {index} failed with code {}: {}",
                    write_error.code, write_error.message
                );
            }
            if let Some(partial) = &failure.partial_result {
                println!("partial result: {partial:?}");
            }
        }
    }
}
```

The full driver error also remains available through `std::error::Error::source`.

### One model per batch

A `BulkWrite<M>` batch targets one model and therefore one collection. MongoDB's `bulkWrite` can span namespaces; for cross-namespace batches — or driver capabilities outside the typed surface, such as raw update pipelines — use `mongodb::Client::bulk_write` directly. That escape hatch bypasses OxiMod validation and typed field-name checking for those operations.

---

## Aggregation

Import `Queryable` to call `ModelType::aggregate()`:

```rust
use oximod::Queryable;
```

`aggregate()` starts an ordered aggregation pipeline for the model's collection. Every stage method appends exactly at its call position; OxiMod never reorders, merges, or rewrites the pipeline, because MongoDB evaluates aggregation stages strictly in order.

```rust
let adults = User::aggregate()
    .match_(|user| user.active.eq(true) & user.age.gte(18))
    .sort_by(|user| user.age.desc().then(user.name.asc()))
    .limit(20)
    .all()
    .await?;
```

### Typed stages

* `match_(...)` appends a `$match` stage built from the same typed expressions as `query().filter(...)`. Repeated calls append repeated stages in call order; they are never merged.
* `sort_by(...)` appends a `$sort` stage. One stage carries several keys by chaining `then`: `user.role.asc().then(user.age.desc())`. Calling `sort_by` again appends another stage rather than replacing the first, because sort position is pipeline semantics.
* `skip(n)` and `limit(n)` append `$skip` and `$limit` stages. Unlike `Query`, these are ordered stages, not query-wide modifiers.
* `text(...)` appends a `$match` stage containing `$text`. MongoDB requires it to be the pipeline's first stage and the collection needs a text index.

Typed stages use serialized field names, following supported Serde renames, exactly like typed queries. MongoDB forbids some query operators inside an aggregation `$match`, notably `$near` and `$nearSphere`; geospatial distance pipelines use a raw `$geoNear` stage as the first pipeline stage instead.

### Raw stages

The rest of MongoDB's aggregation language — `$group`, `$project`, `$geoNear`, `$lookup`, `$unwind`, `$facet`, and everything else — is reached through raw stages:

```rust
use mongodb::bson::doc;

let summaries = User::aggregate()
    .match_(|user| user.active.eq(true))
    .raw_stage_with(|user| {
        doc! {
            "$group": {
                "_id": format!("${}", user.role.name()),
                "count": { "$sum": 1 },
            },
        }
    })
    .with_type::<RoleSummary>()
    .all()
    .await?;
```

* `raw_stage(doc! { ... })` appends one stage document unchanged.
* `raw_stage_with(|fields| doc! { ... })` is the same, but the closure receives the generated fields, so source field references built from `fields.<name>.name()` stay compiler-linked — a renamed or removed model field becomes a compile error instead of a silently broken pipeline.
* `raw_pipeline([...])` appends several stage documents in order.

Raw stage BSON is not operator-checked or path-checked by OxiMod, and MongoDB's server rules still apply: `$geoNear` must be the first stage, and the write stages `$out` and `$merge` are restricted inside transactions. A server rejection surfaces as `OxiModError::Aggregation`.

### Output types

The source model and the output type are distinct concepts: a pipeline may preserve the model shape, extend it, or replace it entirely.

* `$match`, `$sort`, `$skip`, and `$limit` preserve the model shape, so results deserialize as the model by default.
* `$addFields` may preserve the model shape if the extra fields are ignored during deserialization; use a dedicated output type when the added data matters.
* `$group` and `$project` typically replace the shape entirely and require `with_type`.

```rust
#[derive(Debug, serde::Deserialize)]
struct RoleSummary {
    #[serde(rename = "_id")]
    role: String,
    count: i64,
}
```

`with_type::<R>()` changes only how output deserializes; it adds no stage. OxiMod cannot verify at compile time that an output struct matches what the pipeline produces — after a shape-changing raw stage, that synchronization is the caller's responsibility. If an output document does not deserialize as the selected type, execution fails with `OxiModError::Serialization` rather than silently dropping or defaulting values, and one undecodable document fails the whole `all()` call.

After a shape-changing raw stage, use typed source-field stages only if those source fields still exist with the same meaning; otherwise continue with raw stages and set the final output type explicitly.

### Execution

```rust
let users = User::aggregate().match_(|user| user.active.eq(true)).all().await?;
let first = User::aggregate().sort_by(|user| user.age.desc()).limit(1).first().await?;

let users = User::aggregate().match_(|user| user.active.eq(true)).all_from(&client).await?;
let users = User::aggregate().match_(|user| user.active.eq(true)).all_with_session(&mut session).await?;
```

* `all()` / `first()` execute through the global client.
* `all_from(&client)` / `first_from(&client)` execute through an explicit `mongodb::Client`, so aggregation does not require global initialization.
* `all_with_session(&mut session)` / `first_with_session(&mut session)` resolve the collection from the session's own client and advance the cursor with the same session, so inside a transaction the pipeline sees the transaction's own uncommitted writes. Session participation is always explicit.

`first()` runs the pipeline exactly as built and reads at most one result from the cursor; it does not append a server-side `$limit: 1`, because rewriting the pipeline could change raw-stage behavior. Add an explicit `.limit(1)` stage to stop the server early.

There is no streaming terminal; `all()` materializes results the same way `query().all()` does. For cursor streaming or database-level aggregation, use the raw collection escape hatch below.

### Driver options

```rust
use mongodb::options::AggregateOptions;

let options = AggregateOptions::builder().allow_disk_use(true).build();

let users = User::aggregate()
    .match_(|user| user.active.eq(true))
    .with_options(options)
    .all()
    .await?;
```

`with_options` passes the driver's `AggregateOptions` through unchanged — batch size, max time, collation, comment, hint, `let` variables, and read/write concerns remain driver and server behavior — and applies identically to global, explicit-client, and session execution.

### Aggregation errors

* connectivity failure during aggregation → `OxiModError::Connection`;
* output BSON decode failure → `OxiModError::Serialization`;
* every other aggregation server/driver operational failure, such as a rejected pipeline → `OxiModError::Aggregation`, with the original `mongodb::error::Error` preserved as `source()`.

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

### Session-aware counterparts

Persistence operations also have explicit session-aware counterparts:

* `save_with_session()`;
* `save_mut_with_session()`;
* `find_by_id_with_session()`;
* `update_by_id_with_session()`;
* `delete_by_id_with_session()`;
* `exists_with_session()`;
* `count_with_session()`;
* `clear_with_session()`.

These methods accept `&mut mongodb::ClientSession`, resolve the collection from the session's own client, and participate in any transaction active on that session. See [Sessions and transactions](#sessions-and-transactions).

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

Methods without an `_from` or `_with_session` suffix and the non-session typed-query execution methods use this client.

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

Typed query builders execute through the global client, except for the `_with_session` execution terminals, which run through the supplied session's own client. There is no other explicit-client typed-query executor. In an explicit-client workflow without a session, use:

* the `_from` model helpers;
* `get_collection_from()`;
* `get_document_collection_from()`.

The aggregation builder does not share this limitation: `all_from()` and `first_from()` execute an aggregation through an explicit client.

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
* aggregation needs outside the builder, such as cursor streaming or unsupported driver features;
* driver-specific options;
* session usage beyond OxiMod's `_with_session` methods;
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

### Raw aggregation escape hatch

Common aggregation pipelines are covered by the first-class builder described in [Aggregation](#aggregation). The raw collection remains the route for aggregation behavior the builder does not represent — streaming a cursor instead of materializing results, database-level pipelines, or driver features OxiMod does not wrap:

```rust
use futures_util::TryStreamExt;
use mongodb::bson::doc;

let collection = User::get_collection()?;

let pipeline = vec![
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

Raw filters and pipelines must use serialized MongoDB field names. Unlike typed queries and typed aggregation stages, the compiler cannot verify those paths or operator compatibility.

---

## Sessions and transactions

Model operations and typed-query execution terminals have explicit session-aware counterparts ending in `_with_session`. Each takes `&mut mongodb::ClientSession`, resolves the model collection from the session's own client, and participates in any transaction active on that session while keeping OxiMod's typed query construction, validation, and error classification.

Session and transaction lifecycle — `start_session`, `start_transaction`, `commit_transaction`, `abort_transaction`, and any retry handling — belongs to the MongoDB driver; OxiMod does not wrap or retry it.

Model helpers: `save_with_session`, `save_mut_with_session`, `find_by_id_with_session`, `update_by_id_with_session`, `delete_by_id_with_session`, `exists_with_session`, `count_with_session`, and `clear_with_session`.

Typed-query terminals: `first_with_session`, `all_with_session`, `count_with_session`, `update_one_with_session`, `update_all_with_session`, `delete_one_with_session`, and `delete_all_with_session`. Filters, sorting, pagination, array filters, and the multi-document modifier preflight behave exactly as in the non-session terminals.

Bulk-write terminals: `execute_with_session` and `execute_verbose_with_session` run the whole batch as one driver bulk-write action inside the session's transaction. Queued whole-model validation still runs before anything is sent, hooks still do not run, and indexes are never established — initialize them before transactional work.

Aggregation terminals: `all_with_session` and `first_with_session` run the pipeline on the session and advance the result cursor with the same session, so a transaction's own uncommitted writes are visible to the pipeline. MongoDB restricts the write stages `$out` and `$merge` inside transactions; a raw stage violating those rules is rejected by the server as `OxiModError::Aggregation`.

```rust
use oximod::{Model, Queryable};

// Establish declared indexes before transactional work begins.
Order::init_indexes_from(&client).await?;
Inventory::init_indexes_from(&client).await?;

let mut session = client.start_session().await?;
session.start_transaction().await?;

Order::new().sku("sku-1").save_with_session(&mut session).await?;

Inventory::query()
    .filter(|inventory| inventory.sku.eq("sku-1"))
    .update_one_with_session(&mut session, |inventory| {
        inventory.available.inc(-1)
    })
    .await?;

session.commit_transaction().await?;
```

Session participation is always explicit. An ordinary OxiMod call made while a transaction is open does **not** join that transaction — it executes without the session and commits independently, with no error or warning. Every operation that is intended to be atomic must receive the same `ClientSession`.

Rules that keep transactional code correct:

* initialize declared indexes before starting transactional work — session-aware saves never establish indexes, so MongoDB's regular index enforcement applies inside the transaction without hidden out-of-transaction writes;
* `save_with_session` and `save_mut_with_session` retain model validation; typed session-aware updates remain non-validating exactly like their non-session counterparts;
* session-aware reads observe the transaction's own uncommitted writes; sessionless reads do not;
* lifecycle hooks fire in the existing order, but hook callbacks do not receive the caller's session — a database write performed inside a hook is not part of the transaction; perform transactional side writes explicitly in the transaction body instead;
* a post-hook for a session-aware operation runs after that MongoDB operation succeeded in the session, not after the transaction committed;
* do not run parallel MongoDB operations on one session.

Transactions require a MongoDB deployment that supports them, such as a replica set. OxiMod adds no atomicity beyond what the MongoDB driver and server guarantee.

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

Explicit initialization reuses the save path's index machinery and shares its once-per-process establishment state: repeated successful calls are harmless, an index-establishment failure returns the same error surface as save-triggered establishment (`OxiModError::Index` for server-side rejections, `Connection` for connectivity failures) and can be retried by a later call or save, and applications that never call these methods keep the existing lazy save-triggered behavior. This is establishment, not drift synchronization: an index dropped or changed externally after a successful initialization is not automatically re-established during the same process.

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

### Index drift detection and reconciliation

OxiMod separates three index concerns. Each is explicit, and none replaces the others.

**Establishment** — `init_indexes()` / `init_indexes_from(&client)` create the declared indexes once per process, sharing state with the lazy save path. Repeated successful calls are no-ops, and establishment never re-reads server state; see above.

**Inspection** — `check_indexes()` / `check_indexes_from(&client)` perform a read-only, point-in-time comparison of the declared `#[index(...)]` specifications against the index metadata MongoDB reports:

```rust
let report = User::check_indexes_from(&client).await?;

if report.has_drift() {
    for status in report.declared() {
        // DeclaredIndexStatus::InSync { expected, actual }
        // DeclaredIndexStatus::Missing { expected }
        // DeclaredIndexStatus::Mismatched { expected, candidates }
    }
}
```

Each declaration is classified, in declaration order, as:

* `InSync` — a server index is semantically equivalent to the declaration under its effective name. The comparison normalizes MongoDB's listing shape: numeric key types, text indexes' internal `_fts`/`_ftsx` keys (the logical fields are reconstructed from `weights`), server-materialized defaults such as text languages and collation fields, and options whose omission delegates to the server default. An index-level collation wins; otherwise a declaration inherits the collection's default collation, except for text and `2d` indexes, which only support simple binary comparison.
* `Missing` — no server index corresponds to the declaration.
* `Mismatched` — a related server index exists (same effective name, or same logical key under another name) but differs semantically; the report lists each candidate with deterministic, human-readable differences. A same-spec index under a different name is `Mismatched`, not `InSync`, because OxiMod's create lifecycle would conflict on the name. A candidate carrying an option OxiMod cannot declare — a partial filter expression, wildcard projection, or storage-engine setting — is likewise `Mismatched`.

Server indexes unrelated to any declaration are listed under `report.unmanaged()` — the built-in `_id_` index excepted. `is_in_sync()` requires only that every **declared** index is `InSync`: direct-driver compound, partial, and other advanced indexes are a supported escape hatch, never drift. Inspection never creates the collection; an absent collection simply reports every declaration `Missing` (and a model without declarations as in sync).

Inspection suits read-only CI or startup gates:

```rust
let report = User::check_indexes_from(&client).await?;
if !report.is_in_sync() {
    return Err("declared indexes have drifted".into());
}
```

**Conservative reconciliation** — `create_missing_indexes()` / `create_missing_indexes_from(&client)` re-inspect, submit **only** the declarations classified `Missing` in one `createIndexes` call, then inspect again:

```rust
let result = User::create_missing_indexes_from(&client).await?;

result.before();               // drift observed before creation
result.attempted_creations();  // only previously Missing declarations
result.after();                // drift observed after creation

if !result.is_in_sync() {
    // remaining Mismatched declarations require manual action
}
```

The mutating path is deliberately named for exactly what it may do. It never drops an index, never hides or unhides one, never calls `collMod`, never converts an index to unique, never changes a TTL or collation in place, and never drops/recreates a mismatched index or removes an unmanaged one. Mixed drift behaves conservatively: a `Missing` declaration is created even while an unrelated declaration stays `Mismatched`. When nothing is missing, no command is sent at all, so a model with zero declarations never creates its collection; when declarations are missing on an absent collection, MongoDB's `createIndexes` creates the collection implicitly. Reconciliation is independent of the `init_indexes()` once-per-process state in both directions: it neither consults nor completes it.

> **Warning:** Creating a missing index is still operationally consequential. Index builds consume resources and briefly hold an exclusive collection lock at the start and end of an optimized build; a unique index build fails when existing data violates uniqueness (surfacing as `OxiModError::Index` with the duplicate-key driver source); and a newly created TTL index can make already expired documents immediately eligible for deletion, which can create server load. Run reconciliation during controlled startup, deployment, or maintenance workflows — not as a hidden request-path side effect.

Both operations are point-in-time, not transactional: another process can create, drop, or change an index between inspection and creation. A concurrent exact-equivalent create is a server-side no-op; a concurrent conflicting change may surface as an index-domain error. OxiMod adds no retry loop — re-run the check for current state, including after a failed reconciliation (a failure does not imply the server is unchanged).

Drift itself is data, not an error: `OxiModError` is returned only when the inspection or creation operation fails, with the existing classification (`Connection` for connectivity, `Serialization` for BSON failures, `Index` for server-side metadata or creation rejections).

Permissions: inspection needs `listCollections` and `listIndexes` (MongoDB's built-in `read` role suffices); reconciliation additionally needs `createIndex` (the built-in `readWrite` role suffices). OxiMod never needs `dropIndex` or `collMod` privileges for this feature.

Scope: the comparison covers the index metadata visible through the normal collection metadata commands. On sharded deployments it is not a replacement for MongoDB's per-shard index-consistency tooling and does not inspect individual shards.


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

| Hook            | Runs for                                             | Behavior                                                                             |
| --------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `pre_save`      | `save`, `save_from`, `save_with_session`             | Immutable check before validation and insertion.                                     |
| `post_save`     | `save`, `save_from`, `save_with_session`             | Runs after insertion.                                                                |
| `pre_save_mut`  | `save_mut`, `save_from_mut`, `save_mut_with_session` | May mutate the model before validation and insertion.                                |
| `post_save_mut` | `save_mut`, `save_from_mut`, `save_mut_with_session` | May mutate in-memory state after insertion; changes are not automatically persisted. |

### `_id` helper hooks

| Hook                         | Runs for                                                          |
| ---------------------------- | ----------------------------------------------------------------- |
| `pre_find` / `post_find`     | `find_by_id`, `find_by_id_from`, `find_by_id_with_session`        |
| `pre_update` / `post_update` | `update_by_id`, `update_by_id_from`, `update_by_id_with_session`  |
| `pre_delete` / `post_delete` | `delete_by_id`, `delete_by_id_from`, `delete_by_id_with_session`  |

### Hook boundaries

Hooks do **not** wrap:

* typed-query reads, updates, or deletions;
* direct typed or raw collection operations;
* `clear`;
* `exists`;
* `count`;
* collection accessors.

A pre-hook error prevents the associated database operation. A post-hook error is returned after the database operation has already succeeded.

Hook callbacks never receive a `ClientSession`. When a `_with_session` helper fires hooks, a database operation initiated inside a hook executes without the session and is therefore **not** part of the caller's transaction; perform transactional side writes explicitly in the transaction body instead. A post-hook for a session-aware helper runs after that MongoDB operation succeeded in the session — not after the transaction committed — so an aborted transaction rolls back the operation even though its post-hook already ran.

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

Most OxiMod operations return `OxiModError`. Its variants identify failure classes: MongoDB driver errors produced while executing OxiMod database operations are classified by failure class rather than by the method that was executing.

Operation-time driver failures classify with a fixed precedence:

1. `Connection` — MongoDB client/connectivity infrastructure failure: connection establishment, authentication, DNS resolution, TLS configuration, server selection, transport I/O, and connection-pool failure, during any operation (including index establishment).
2. `Serialization` — BSON encoding or decoding failure, in either direction, through every read and write path.
3. `Index` / `Aggregation` / `BulkWrite` — remaining failures of the corresponding operation domain, such as MongoDB rejecting a conflicting index specification, an invalid aggregation pipeline, or a bulk write (including individual write failures within a batch and a server too old for `bulkWrite`).
4. `Database` — every remaining MongoDB/driver operation failure, including duplicate-key rejections. This is the conservative non-connectivity fallback.

A duplicate key **inside a bulk-write batch** therefore classifies as `BulkWrite` (the bulk-write domain refines it), while the same rejection through `save()` remains `Database`. Bulk-write failures preserve per-operation indexes and partial results — see [Bulk writes](#bulk-writes).

`Connection` classifies the failure only: it does not mean the operation never reached MongoDB, and it does not make retrying safe. Retry safety depends on the specific operation, its idempotency, and application policy. `Database` likewise does not guarantee that the server definitely received or rejected the operation.

MongoDB client construction and setup remain a connection concern reported directly as `Connection`, outside the operation-time classifier. `GlobalClientInit`, `GlobalClientMissing`, `Validation`, `Custom`, and `Query` keep their lifecycle, validation, user-defined, and typed-query meanings and are not selected by the operation-time driver classifier.

Driver-backed variants retain the original `mongodb::error::Error` as their `source()`; downcast it for server detail. `Display` text carries human-readable operation context and is not a classification API — do not branch on error message strings.

### Detecting duplicate keys

A duplicate-key rejection classifies as `Database` through `save()` and the update paths alike, with server code 11000 recoverable from the preserved driver error. Duplicate-key failures may surface through the driver as a write error for plain writes or as a command error for findAndModify-based operations such as `query().update_one()`:

```rust
use std::error::Error as _;

use mongodb::error::{ErrorKind, WriteFailure};
use oximod::OxiModError;

fn is_duplicate_key(error: &OxiModError) -> bool {
    matches!(error, OxiModError::Database { .. })
        && error
            .source()
            .and_then(|source| source.downcast_ref::<mongodb::error::Error>())
            .is_some_and(|driver| match &*driver.kind {
                ErrorKind::Write(WriteFailure::WriteError(write_error)) => {
                    write_error.code == 11000
                }
                ErrorKind::Command(command_error) => command_error.code == 11000,
                _ => false,
            })
}
```

### Migrating variant matchers from 0.3.0

OxiMod 0.3.0 selected error variants by call site. Code that matches `OxiModError` variants — exhaustively or otherwise — may observe different arms after the failure-class contract:

| Failure | Path | 0.3.0 variant | Current variant |
|---|---|---|---|
| Duplicate key | `save` / `save_mut` / `save_from` / `save_from_mut` | `Connection` | `Database` |
| Client-side BSON encoding | `save*` | `Connection` | `Serialization` |
| Unreachable server | non-save operations and typed queries | `Database` | `Connection` |
| Unreachable server | index establishment | `Index` | `Connection` |
| Undeserializable document | `find_by_id`, `query().first()` | `Database` | `Serialization` |

Unchanged mappings: an unreachable server through `save*` remains `Connection`; a duplicate key through the update paths remains `Database`; an undeserializable document through `query().all()` remains `Serialization`; index-spec rejections remain `Index`; and the `GlobalClient*`, `Validation`, `Custom`, and `Query` variants are unaffected. `Display` prefixes changed wherever the variant changed; duplicate-key detection keyed on `Connection` around `save()` must switch to the `Database` + `source()` route above, and outage handling keyed on `Database` should match `Connection` instead.

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
* unsupported query modifiers on multi-document (`update_all` / `delete_all`) and bulk-write operations, reported as `QueryError::UnsupportedBulkWriteModifier` naming the operation (`BulkWriteOperation`) and the rejected modifier (`QueryModifier`).

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
| Batch mixed writes into one bulk execution         | `ModelType::bulk_write()` builder    |
| Cross-namespace or raw-pipeline bulk writes        | `mongodb::Client::bulk_write`        |
| Aggregation pipelines                              | `Queryable::aggregate()` builder     |
| Streaming or database-level aggregation            | Direct MongoDB collection access     |
| Compound, partial/filtered, or unsupported index options | MongoDB driver index API       |
| Transactional model and typed-query operations     | `_with_session` methods              |
| Advanced session and driver features               | Direct MongoDB collection/client API |

OxiMod is designed so these approaches can coexist in the same application.

---

## Examples

The repository includes focused runnable examples covering:

* aggregation;
* basic persistence;
* bulk writes;
* `_id` workflows;
* custom validation;
* defaults;
* typed deletion;
* lifecycle hooks;
* raw MongoDB queries;
* typed queries;
* typed updates;
* explicit-client workflows;
* index drift inspection and create-only reconciliation;
* structured validation errors;
* built-in validation;
* nested embedded-model validation.

Browse them in [`oximod/examples`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples).

Run an example with:

```bash
cargo run -p oximod --example typed_query
```

MongoDB-backed examples read `MONGODB_URI` from the environment or a `.env` file.

---

## Current behavioral notes

* Typed-query execution requires the global client, except for the `_with_session` terminals, which use the session's own client.
* Typed and raw update operations do not automatically run model validation.
* Validation descends into embedded models only where the containing field opts in with `#[validate(nested)]`; fields without the attribute do not evaluate embedded rules through the parent.
* Generated indexes are single-field and are initialized lazily during saves, or explicitly at startup with `init_indexes()` / `init_indexes_from(&client)`; initialization is once per process and does not re-establish indexes dropped externally afterward.
* Compound and partial/filtered indexes require the MongoDB driver API; a derived composite-key field with `#[index(unique)]` is not a safe substitute for a compound unique index.
* Session participation is explicit through the `_with_session` methods; a non-session OxiMod call issued while a transaction is open commits outside that transaction. Initialize declared indexes before transactional work — session-aware saves do not establish them.
* Typed reads fail as a whole when any document in the selected result window cannot be deserialized; use the raw document collection to inspect or repair such documents.
* Lifecycle hooks wrap only save and `_id` helper methods; bulk-write operations never invoke them.
* Bulk writes require MongoDB Server 8.0+, validate queued whole-model inserts and replacements before any network communication, and preserve the driver's per-operation failure indexes and partial results through `OxiModError::BulkWrite`.
* `clear()`, unfiltered `update_all()`, unfiltered `delete_all()`, and unfiltered bulk `update_many`/`delete_many` operations can affect an entire collection.
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

