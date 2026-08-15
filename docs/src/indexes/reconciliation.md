# Drift Detection and Reconciliation

OxiMod separates three index concerns. Each is explicit, and none replaces the
others.

**Establishment** — `init_indexes()` / `init_indexes_from(&client)` create the
declared indexes once per process, sharing state with the lazy save path.
Repeated successful calls are no-ops, and establishment never re-reads server
state; see [Establishment and Lifecycle](lifecycle.md).

**Inspection** — `check_indexes()` / `check_indexes_from(&client)` perform a
read-only, point-in-time comparison of the declared `#[index(...)]`
specifications against the index metadata MongoDB reports.

**Conservative reconciliation** — `create_missing_indexes()` /
`create_missing_indexes_from(&client)` create only the declarations found
missing, and nothing else.

## Inspection

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

* `InSync` — a server index is semantically equivalent to the declaration
  under its effective name. The comparison normalizes MongoDB's listing shape:
  numeric key types, text indexes' internal `_fts`/`_ftsx` keys (the logical
  fields are reconstructed from `weights`), server-materialized defaults such
  as text languages and collation fields, and options whose omission delegates
  to the server default. An index-level collation wins; otherwise a
  declaration inherits the collection's default collation, except for text and
  `2d` indexes, which only support simple binary comparison.
* `Missing` — no server index corresponds to the declaration.
* `Mismatched` — a related server index exists (same effective name, or same
  logical key under another name) but differs semantically; the report lists
  each candidate with deterministic, human-readable differences. A same-spec
  index under a different name is `Mismatched`, not `InSync`, because OxiMod's
  create lifecycle would conflict on the name. A candidate carrying an option
  OxiMod cannot declare — a partial filter expression, wildcard projection, or
  storage-engine setting — is likewise `Mismatched`.

Server indexes unrelated to any declaration are listed under
`report.unmanaged()` — the built-in `_id_` index excepted. `is_in_sync()`
requires only that every **declared** index is `InSync`: direct-driver
compound, partial, and other advanced indexes are a supported escape hatch,
never drift. Inspection never creates the collection; an absent collection
simply reports every declaration `Missing` (and a model without declarations
as in sync).

Inspection suits read-only CI or startup gates:

```rust
let report = User::check_indexes_from(&client).await?;
if !report.is_in_sync() {
    return Err("declared indexes have drifted".into());
}
```

## Conservative reconciliation

`create_missing_indexes()` / `create_missing_indexes_from(&client)`
re-inspect, submit **only** the declarations classified `Missing` in one
`createIndexes` call, then inspect again:

```rust
let result = User::create_missing_indexes_from(&client).await?;

result.before();               // drift observed before creation
result.attempted_creations();  // only previously Missing declarations
result.after();                // drift observed after creation

if !result.is_in_sync() {
    // remaining Mismatched declarations require manual action
}
```

The mutating path is deliberately named for exactly what it may do. It never
drops an index, never hides or unhides one, never calls `collMod`, never
converts an index to unique, never changes a TTL or collation in place, and
never drops/recreates a mismatched index or removes an unmanaged one. Mixed
drift behaves conservatively: a `Missing` declaration is created even while an
unrelated declaration stays `Mismatched`. When nothing is missing, no command
is sent at all, so a model with zero declarations never creates its
collection; when declarations are missing on an absent collection, MongoDB's
`createIndexes` creates the collection implicitly. Reconciliation is
independent of the `init_indexes()` once-per-process state in both directions:
it neither consults nor completes it.

> **Warning:** Creating a missing index is still operationally consequential.
> Index builds consume resources and briefly hold an exclusive collection lock
> at the start and end of an optimized build; a unique index build fails when
> existing data violates uniqueness (surfacing as `OxiModError::Index` with
> the duplicate-key driver source); and a newly created TTL index can make
> already expired documents immediately eligible for deletion, which can
> create server load. Run reconciliation during controlled startup,
> deployment, or maintenance workflows — not as a hidden request-path side
> effect.

## Races, errors, permissions, and scope

Both operations are point-in-time, not transactional: another process can
create, drop, or change an index between inspection and creation. A concurrent
exact-equivalent create is a server-side no-op; a concurrent conflicting
change may surface as an index-domain error. OxiMod adds no retry loop —
re-run the check for current state, including after a failed reconciliation (a
failure does not imply the server is unchanged).

Drift itself is data, not an error: `OxiModError` is returned only when the
inspection or creation operation fails, with the existing classification
(`Connection` for connectivity, `Serialization` for BSON failures, `Index` for
server-side metadata or creation rejections).

Permissions: inspection needs `listCollections` and `listIndexes` (MongoDB's
built-in `read` role suffices); reconciliation additionally needs
`createIndex` (the built-in `readWrite` role suffices). OxiMod never needs
`dropIndex` or `collMod` privileges for this feature.

Scope: the comparison covers the index metadata visible through the normal
collection metadata commands. On sharded deployments it is not a replacement
for MongoDB's per-shard index-consistency tooling and does not inspect
individual shards.

## Related material

* Runnable workflow: `index_reconciliation` in
  [Runnable Examples](../reference/examples.md).
