# Declaring Indexes

Declare a single-field MongoDB index directly on a collection-model field:

```rust
#[index(unique, name = "email_idx")]
email: String,
```

Declared indexes are not created by deriving the model; when and how they are
created is covered in [Establishment and Lifecycle](lifecycle.md).

## Core options

| Option                     | Description                                      |
| -------------------------- | ------------------------------------------------ |
| `unique`                   | Enforces unique values.                          |
| `sparse`                   | Excludes documents where the field is missing.   |
| `hidden`                   | Hides the index from the query planner.          |
| `name = "..."`             | Assigns an explicit index name.                  |
| `order = 1` / `order = -1` | Creates an ascending or descending scalar index. |
| `expire_after_secs = N`    | Creates a TTL index.                             |
| `background`               | Forwards MongoDB's background option.            |

## Specialized index types

| Option         | Description                     |
| -------------- | ------------------------------- |
| `text`         | Creates a text index.           |
| `hashed`       | Creates a hashed index.         |
| `wildcard`     | Creates a wildcard field index. |
| `geo_2dsphere` | Creates a `2dsphere` index.     |
| `geo_2d`       | Creates a planar `2d` index.    |

## Advanced options

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

Text-specific options such as `weight`, `default_language`,
`language_override`, and `text_index_version` imply a text index.

## Examples

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

## What `#[index(...)]` cannot express

Use direct collection access for compound indexes and advanced options not
represented by `#[index(...)]`.

Partial or filtered indexes (MongoDB's `partialFilterExpression` option) are
likewise not expressible with `#[index(...)]`; create them through the
driver's `create_index` on the collection returned by `get_collection()` or
`get_document_collection()`. Driver-created indexes coexist with
`#[index(...)]` declarations. MongoDB enforces that uniqueness; the underlying
driver failure on a violation is a duplicate-key error (E11000), not an OxiMod
validation failure — OxiMod validation does not replace MongoDB's index
enforcement.

> **Warning:** Do not emulate a compound unique index by storing a derived
> composite-key field guarded by `#[index(unique)]`. Partial updates such as
> `update_by_id` or a typed `$set` can change the source fields without
> recomputing the derived field, silently desynchronizing it; genuine
> duplicates can then persist while the index still appears healthy. Create a
> real MongoDB compound unique index through the driver instead.
