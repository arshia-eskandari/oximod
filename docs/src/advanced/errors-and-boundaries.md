# Errors and Behavioral Boundaries

Most OxiMod operations return `OxiModError`. Its variants identify failure
classes: MongoDB driver errors produced while executing OxiMod database
operations are classified by failure class rather than by the method that was
executing.

## Failure-class precedence

Operation-time driver failures classify with a fixed precedence:

1. `Connection` — MongoDB client/connectivity infrastructure failure:
   connection establishment, authentication, DNS resolution, TLS
   configuration, server selection, transport I/O, and connection-pool
   failure, during any operation (including index establishment).
2. `Serialization` — BSON encoding or decoding failure, in either direction,
   through every read and write path.
3. `Index` / `Aggregation` / `BulkWrite` — remaining failures of the
   corresponding operation domain, such as MongoDB rejecting a conflicting
   index specification, an invalid aggregation pipeline, or a bulk write
   (including individual write failures within a batch and a server too old
   for `bulkWrite`).
4. `Database` — every remaining MongoDB/driver operation failure, including
   duplicate-key rejections. This is the conservative non-connectivity
   fallback.

A duplicate key **inside a bulk-write batch** therefore classifies as
`BulkWrite` (the bulk-write domain refines it), while the same rejection
through `save()` remains `Database`. Bulk-write failures preserve
per-operation indexes and partial results — see
[Bulk Writes](../operations/bulk-writes.md).

`Connection` classifies the failure only: it does not mean the operation never
reached MongoDB, and it does not make retrying safe. Retry safety depends on
the specific operation, its idempotency, and application policy. `Database`
likewise does not guarantee that the server definitely received or rejected
the operation.

MongoDB client construction and setup remain a connection concern reported
directly as `Connection`, outside the operation-time classifier.
`GlobalClientInit`, `GlobalClientMissing`, `Validation`, `Custom`, and `Query`
keep their lifecycle, validation, user-defined, and typed-query meanings and
are not selected by the operation-time driver classifier.

Driver-backed variants retain the original `mongodb::error::Error` as their
`source()`; downcast it for server detail. `Display` text carries
human-readable operation context and is not a classification API — do not
branch on error message strings.

## Detecting duplicate keys

A duplicate-key rejection classifies as `Database` through `save()` and the
update paths alike, with server code 11000 recoverable from the preserved
driver error. Duplicate-key failures may surface through the driver as a write
error for plain writes or as a command error for findAndModify-based
operations such as `query().update_one()`:

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

## Migrating variant matchers from 0.3.0

OxiMod 0.3.0 selected error variants by call site. Code that matches
`OxiModError` variants — exhaustively or otherwise — may observe different
arms after the failure-class contract:

| Failure | Path | 0.3.0 variant | Current variant |
|---|---|---|---|
| Duplicate key | `save` / `save_mut` / `save_from` / `save_from_mut` | `Connection` | `Database` |
| Client-side BSON encoding | `save*` | `Connection` | `Serialization` |
| Unreachable server | non-save operations and typed queries | `Database` | `Connection` |
| Unreachable server | index establishment | `Index` | `Connection` |
| Undeserializable document | `find_by_id`, `query().first()` | `Database` | `Serialization` |

Unchanged mappings: an unreachable server through `save*` remains
`Connection`; a duplicate key through the update paths remains `Database`; an
undeserializable document through `query().all()` remains `Serialization`;
index-spec rejections remain `Index`; and the `GlobalClient*`, `Validation`,
`Custom`, and `Query` variants are unaffected. `Display` prefixes changed
wherever the variant changed; duplicate-key detection keyed on `Connection`
around `save()` must switch to the `Database` + `source()` route above, and
outage handling keyed on `Database` should match `Connection` instead.

## Validation errors

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

Several rules may produce several messages for the same field. For
`#[validate(nested)]` descent, `field` holds a nested path such as
`address.postal_code`; see
[Nested error paths](../models/validation.md#nested-error-paths).

## Query errors

Typed-query configuration failures are exposed through `OxiModError::Query`
and `query_error()`. They include:

* zero page numbers;
* zero page sizes;
* pagination overflow;
* limits outside the driver's supported integer range;
* unsupported query modifiers on multi-document (`update_all` / `delete_all`)
  and bulk-write operations, reported as
  `QueryError::UnsupportedBulkWriteModifier` naming the operation
  (`BulkWriteOperation`) and the rejected modifier (`QueryModifier`).

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

## Behavioral boundaries at a glance

Each of these boundaries is covered in depth by its feature chapter; this list
collects them for review:

* Typed-query execution requires the global client, except for the
  `_with_session` terminals, which use the session's own client.
* Typed and raw update operations do not automatically run model validation.
* Validation descends into embedded models only where the containing field
  opts in with `#[validate(nested)]`; fields without the attribute do not
  evaluate embedded rules through the parent.
* Generated indexes are single-field and are initialized lazily during saves,
  or explicitly at startup with `init_indexes()` /
  `init_indexes_from(&client)`; initialization is once per process and does
  not re-establish indexes dropped externally afterward.
* Compound and partial/filtered indexes require the MongoDB driver API; a
  derived composite-key field with `#[index(unique)]` is not a safe substitute
  for a compound unique index.
* Session participation is explicit through the `_with_session` methods; a
  non-session OxiMod call issued while a transaction is open commits outside
  that transaction. Initialize declared indexes before transactional work —
  session-aware saves do not establish them.
* Typed reads fail as a whole when any document in the selected result window
  cannot be deserialized; use the raw document collection to inspect or repair
  such documents.
* Lifecycle hooks wrap only save and `_id` helper methods; bulk-write
  operations never invoke them.
* Bulk writes require MongoDB Server 8.0+, validate queued whole-model inserts
  and replacements before any network communication, and preserve the driver's
  per-operation failure indexes and partial results through
  `OxiModError::BulkWrite`.
* `clear()`, unfiltered `update_all()`, unfiltered `delete_all()`, and
  unfiltered bulk `update_many`/`delete_many` operations can affect an entire
  collection.
* `GeoPoint`, `GeoPolygon`, and `NearQuery` construct MongoDB geometry and
  query documents but do not perform complete geospatial validity checks.
* `index_max_retries` and `index_max_init_seconds` are accepted but are not
  currently enforced as hard limits.
* Retry behavior remains the MongoDB driver's responsibility; OxiMod adds no
  retry loop anywhere.

## Related material

* Runnable workflow: `validate_extract_errors` in
  [Runnable Examples](../reference/examples.md).
