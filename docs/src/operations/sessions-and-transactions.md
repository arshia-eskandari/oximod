# Sessions and Transactions

Model operations and typed-query execution terminals have explicit
session-aware counterparts ending in `_with_session`. Each takes
`&mut mongodb::ClientSession`, resolves the model collection from the
session's own client, and participates in any transaction active on that
session while keeping OxiMod's typed query construction, validation, and error
classification.

Session and transaction lifecycle — `start_session`, `start_transaction`,
`commit_transaction`, `abort_transaction`, and any retry handling — belongs to
the MongoDB driver; OxiMod does not wrap or retry it.

## Session-aware surface

Model helpers: `save_with_session`, `save_mut_with_session`,
`find_by_id_with_session`, `update_by_id_with_session`,
`delete_by_id_with_session`, `exists_with_session`, `count_with_session`, and
`clear_with_session`.

Typed-query terminals: `first_with_session`, `all_with_session`,
`count_with_session`, `update_one_with_session`, `update_all_with_session`,
`delete_one_with_session`, and `delete_all_with_session`. Filters, sorting,
pagination, array filters, and the multi-document modifier preflight behave
exactly as in the non-session terminals.

Bulk-write terminals: `execute_with_session` and
`execute_verbose_with_session` run the whole batch as one driver bulk-write
action inside the session's transaction. Queued whole-model validation still
runs before anything is sent, hooks still do not run, and indexes are never
established — initialize them before transactional work. See
[Bulk Writes](bulk-writes.md).

Aggregation terminals: `all_with_session` and `first_with_session` run the
pipeline on the session and advance the result cursor with the same session,
so a transaction's own uncommitted writes are visible to the pipeline. MongoDB
restricts the write stages `$out` and `$merge` inside transactions; a raw
stage violating those rules is rejected by the server as
`OxiModError::Aggregation`.

## A transactional workflow

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

## Session participation is always explicit

An ordinary OxiMod call made while a transaction is open does **not** join
that transaction — it executes without the session and commits independently,
with no error or warning. Every operation that is intended to be atomic must
receive the same `ClientSession`.

Rules that keep transactional code correct:

* initialize declared indexes before starting transactional work —
  session-aware saves never establish indexes, so MongoDB's regular index
  enforcement applies inside the transaction without hidden out-of-transaction
  writes;
* `save_with_session` and `save_mut_with_session` retain model validation;
  typed session-aware updates remain non-validating exactly like their
  non-session counterparts;
* session-aware reads observe the transaction's own uncommitted writes;
  sessionless reads do not;
* lifecycle hooks fire in the existing order, but hook callbacks do not
  receive the caller's session — a database write performed inside a hook is
  not part of the transaction; perform transactional side writes explicitly in
  the transaction body instead;
* a post-hook for a session-aware operation runs after that MongoDB operation
  succeeded in the session, not after the transaction committed;
* do not run parallel MongoDB operations on one session.

Transactions require a MongoDB deployment that supports them, such as a
replica set. OxiMod adds no atomicity beyond what the MongoDB driver and
server guarantee.

## Related material

* Hook/session boundaries in detail: [Lifecycle Hooks](../advanced/hooks.md).
* Runnable workflow: `session_transactions` in
  [Runnable Examples](../reference/examples.md) (requires a replica set).
