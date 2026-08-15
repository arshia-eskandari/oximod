# Collection and Embedded Models

OxiMod uses the same `Model` derive for two distinct model kinds.

## Collection-backed models

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
* lazy index initialization, with explicit `init_indexes()` startup
  initialization;
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

## Embedded models

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

## Querying embedded values

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

Generated field paths honor supported Serde `rename` and `rename_all`
attributes, including nested paths. In the `Address` example above, a typed
filter on `street_name` targets the stored `streetName` field.

Nested filtering, sorting, and updates through embedded paths are covered in
[Typed Queries](../operations/typed-queries.md).

## Related chapters

* [Builders, Defaults, and IDs](builders-defaults-and-ids.md) — what the
  generated construction API looks like for both model kinds.
* [Validation](validation.md) — including opt-in nested validation of embedded
  models.
