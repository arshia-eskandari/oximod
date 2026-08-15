# Establishment and Lifecycle

Declared indexes are not created by deriving the model. Generated indexes are
initialized lazily before model insertion. A successful initialization is
remembered for that model type within the process. Merely constructing a query
or obtaining a collection does not create indexes.

## Explicit startup establishment

Applications that need declared indexes before their first write — for example
so a unique constraint is enforced from process start — can establish them
explicitly during startup:

```rust
User::init_indexes().await?;              // global client
User::init_indexes_from(&client).await?;  // explicit client
```

Explicit initialization reuses the save path's index machinery and shares its
once-per-process establishment state: repeated successful calls are harmless,
an index-establishment failure returns the same error surface as
save-triggered establishment (`OxiModError::Index` for server-side rejections,
`Connection` for connectivity failures) and can be retried by a later call or
save, and applications that never call these methods keep the existing lazy
save-triggered behavior.

## Establishment is not drift synchronization

Explicit initialization is establishment, not drift synchronization: an index
dropped or changed externally after a successful initialization is not
automatically re-established during the same process. To observe or repair
differences between declarations and server state, use the explicit inspection
and reconciliation surface described in
[Drift Detection and Reconciliation](reconciliation.md).

Bulk writes never establish indexes, and session-aware saves never establish
them either — initialize declared indexes before bulk ingest or transactional
work. See [Bulk Writes](../operations/bulk-writes.md) and
[Sessions and Transactions](../operations/sessions-and-transactions.md).

## Index-initialization attributes

The derive currently accepts:

```rust
#[index_max_retries(N)]
#[index_max_init_seconds(N)]
```

These values are stored in the generated index coordinator, but the current
runtime does not enforce them as hard retry or timeout limits. Do not rely on
them as operational guarantees yet.
