# Direct Driver Access

OxiMod intentionally preserves access to the official driver.

## Typed collection

```rust
let collection = User::get_collection()?;
```

Returns:

```rust
mongodb::Collection<User>
```

Use it for:

* raw BSON queries with typed deserialization;
* aggregation needs outside the builder, such as cursor streaming or
  unsupported driver features;
* driver-specific options;
* session usage beyond OxiMod's `_with_session` methods;
* operations not represented by OxiMod's helpers.

## Raw document collection

```rust
let collection = User::get_document_collection()?;
```

Returns:

```rust
mongodb::Collection<mongodb::bson::Document>
```

Use it when the document shape is dynamic or when working directly with BSON.

## Raw aggregation escape hatch

Common aggregation pipelines are covered by the first-class builder described
in [Aggregation](../operations/aggregation.md). The raw collection remains the
route for aggregation behavior the builder does not represent — streaming a
cursor instead of materializing results, database-level pipelines, or driver
features OxiMod does not wrap:

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

Raw filters and pipelines must use serialized MongoDB field names. Unlike
typed queries and typed aggregation stages, the compiler cannot verify those
paths or operator compatibility.

## Other driver responsibilities

Direct driver access is a supported escape hatch, not a failure mode. Beyond
collections, the driver remains responsible for:

* compound, partial/filtered, and other advanced indexes (see
  [Declaring Indexes](../indexes/declarations.md));
* cross-namespace bulk writes through `mongodb::Client::bulk_write` (see
  [Bulk Writes](../operations/bulk-writes.md));
* session and transaction lifecycle, and any retry behavior (see
  [Sessions and Transactions](../operations/sessions-and-transactions.md)).

## Related material

* Runnable workflow: `query` (raw-filter usage) in
  [Runnable Examples](../reference/examples.md).
