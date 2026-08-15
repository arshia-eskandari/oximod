# Persistence and Clients

The `Model` trait is implemented only for collection-backed models. It
provides saves, `_id`-based helpers, collection utilities, and direct
collection access, each in global-client, explicit-client, and session-aware
forms.

## Global-client methods

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

## Explicit-client counterparts

Every persistence or collection-access operation has an explicit-client
counterpart:

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

## Session-aware counterparts

Persistence operations also have explicit session-aware counterparts:

* `save_with_session()`;
* `save_mut_with_session()`;
* `find_by_id_with_session()`;
* `update_by_id_with_session()`;
* `delete_by_id_with_session()`;
* `exists_with_session()`;
* `count_with_session()`;
* `clear_with_session()`.

These methods accept `&mut mongodb::ClientSession`, resolve the collection
from the session's own client, and participate in any transaction active on
that session. See
[Sessions and Transactions](sessions-and-transactions.md).

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

Methods without an `_from` or `_with_session` suffix and the non-session
typed-query execution methods use this client.

```rust
let user_id = user.save().await?;
let users = User::query().all().await?;
```

`OxiClient::global()` returns an `Arc<mongodb::Client>` and fails when global
initialization has not completed. A second successful initialization is not
allowed.

Treat `init_global()` as a process-level, one-time startup step and handle its
`Result`. Global-client operations require initialization to have completed
successfully; until it has, they fail. Once a client is installed, later
`init_global()` calls return an error rather than replacing it.

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

`OxiClient::default()` starts without an inner client. Initialize it later
with `init_client()`.

### `OxiClient` API

| Method             | Description                                          |
| ------------------ | ---------------------------------------------------- |
| `new(uri)`         | Creates an initialized instance-level wrapper.       |
| `init_client(uri)` | Initializes or replaces the wrapper's client.        |
| `client()`         | Returns `Option<&mongodb::Client>`.                  |
| `client_mut()`     | Returns `Option<&mut mongodb::Client>`.              |
| `init_global(uri)` | Initializes the process-wide client once.            |
| `global()`         | Returns the shared client as `Arc<mongodb::Client>`. |

## Typed-query limitation

Typed query builders execute through the global client, except for the
`_with_session` execution terminals, which run through the supplied session's
own client. There is no other explicit-client typed-query executor. In an
explicit-client workflow without a session, use:

* the `_from` model helpers;
* `get_collection_from()`;
* `get_document_collection_from()`.

The aggregation builder does not share this limitation: `all_from()` and
`first_from()` execute an aggregation through an explicit client. See
[Aggregation](aggregation.md).

## Related material

* Raw collection access and when to prefer it:
  [Direct Driver Access](../advanced/driver-access.md).
* Runnable workflows: `basic_usage`, `by_id`, and `update_with_client` in
  [Runnable Examples](../reference/examples.md).
