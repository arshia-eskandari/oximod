# Updates and Deletion

## Typed updates

Typed update expressions are built from the same generated fields used for
filters:

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

Combine independent update expressions with `&`. Updates using the same
MongoDB operator are merged into one operator document.

Typed update expressions write specific dotted field paths. When documents
written by older model versions may still be present, prefer these targeted
updates — or `$set` on dotted paths through `update_by_id` — over replacing a
whole stored document with a serialized model: a whole-document replacement
writes only the current struct shape and can drop or rewrite fields the
running code no longer declares.

Supported families include:

* scalar `$set`;
* optional-field `$unset`;
* numeric `$inc`, `$mul`, `$min`, and `$max`;
* field rename and current-date updates;
* array push, set-like addition, pull, and pop operations;
* positional and array-filtered updates for embedded arrays.

Numeric updates on `Option<T>` fields take inner-type operands, exactly as on
required fields.

Typed and raw updates do **not** automatically run whole-model validation; see
[Validation and updates](../models/validation.md#validation-and-updates).

## Single-document update

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

## Multi-document update

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .update_all(|user| user.active.set(true))
    .await?;
```

`update_all()` updates every matching document in one command, returns
MongoDB's `UpdateResult`, and rejects sorting, skipping, limiting, and
pagination instead of silently ignoring them.

To batch several independent write intentions — including inserts and deletes
— into one OxiMod bulk execution, see [Bulk Writes](bulk-writes.md).

> **Warning:** An unfiltered `update_all()` affects every document in the
> collection.

## Single-document deletion

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

## Multi-document deletion

```rust
let result = User::query()
    .filter(|user| user.active.eq(false))
    .delete_all()
    .await?;
```

`delete_all()` deletes every matching document in one command, returns
MongoDB's `DeleteResult`, and rejects sorting, skipping, limiting, and
pagination.

> **Warning:** An unfiltered `delete_all()` affects every document in the
> collection.

## Related material

* Rejected modifiers surface as typed query errors; see
  [Errors and Behavioral Boundaries](../advanced/errors-and-boundaries.md).
* Runnable workflows: `update`, `delete`, and `update_with_client` in
  [Runnable Examples](../reference/examples.md).
