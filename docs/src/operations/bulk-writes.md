# Bulk Writes

`ModelType::bulk_write()` (a `Model` method) queues multiple independent write
intentions against one model's collection and submits them to MongoDB as **one
driver bulk-write action** — never by expanding the batch into per-item OxiMod
save/update/delete calls. (The MongoDB driver may split a sufficiently large
logical batch into multiple server commands to satisfy server/message batch
limits; that splitting belongs to the driver.) The batch may mix all six
operation kinds freely, and OxiMod preserves the queue order exactly:

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

Bulk writes require **MongoDB Server 8.0+**. OxiMod does not emulate the
command on older servers: a server that cannot execute it fails through the
ordinary bulk-write error path, and OxiMod never silently expands a batch into
individual writes. Retryable-write behavior belongs to the MongoDB driver;
OxiMod adds no retry loop.

## Typed construction

Update and delete operations consume ordinary `Query<M>` values, so filters,
text search, and array filters reuse the exact typed construction and
Serde-renamed paths of the rest of the query API, and updates reuse
`UpdateExpression`. Modifiers a driver bulk model cannot represent are
rejected locally — before any network communication — as
`QueryError::UnsupportedBulkWriteModifier`, never silently dropped:

* `update_one` and `replace_one` support a query sort (it selects which
  matching document is written); skip, limit, and pagination are rejected;
* `update_many` supports array filters; sort, skip, limit, and pagination are
  rejected;
* `delete_one` and `delete_many` cannot carry a sort (unlike
  `Query::delete_one`), and reject skip, limit, pagination, and array filters.

## Validation, hooks, and indexes

Whole model values queued through `insert`, `insert_many`, and `replace_one`
run `#[validate]` for **every** queued value before any network communication:
if the 101st queued insert is invalid, the batch returns
`OxiModError::Validation` and zero writes are sent. Typed update expressions
remain non-validating, exactly like `update_one()`/`update_all()`.

Bulk-write operations are a separate execution surface. They preserve typed
query/update construction and whole-model validation where applicable, but
they do **not** invoke OxiMod lifecycle hooks.

Executing a bulk write never establishes declared `#[index(...)]`
specifications. Initialize indexes explicitly — especially unique constraints
used as idempotency guards — with `init_indexes()` / `init_indexes_from(&client)`
before bulk ingest.

Replacements follow ordinary MongoDB semantics: `replace_one` rewrites the
**whole stored document** with the serialized model, removing fields the Rust
model does not declare. Prefer targeted typed updates when documents written
by other application versions may exist.

## Ordered and unordered execution

MongoDB executes bulk writes in order by default and stops after the first
failing operation. With `.ordered(false)`, the server continues attempting the
remaining operations after a failure and may reorder execution for
performance; do not depend on execution order when unordered.
`.with_options(BulkWriteOptions)` passes the driver's full options through
(ordered execution, server-side `bypass_document_validation`, comment, `let`
variables, write concern); `bypass_document_validation` never disables
OxiMod's client-side `#[validate]` preflight.

## Execution and results

Summary terminals return the driver's `SummaryBulkWriteResult` (inserted,
matched, modified, upserted, and deleted counts); `_verbose` terminals return
`VerboseBulkWriteResult` with per-operation results keyed by each operation's
original queued index:

* `execute()` / `execute_verbose()` — global client;
* `execute_from(&client)` / `execute_verbose_from(&client)` — explicit client;
* `execute_with_session(&mut session)` /
  `execute_verbose_with_session(&mut session)` — session/transaction execution
  through the session's own client.

## Partial failures keep their operation indexes

When individual writes fail, the returned `OxiModError::BulkWrite` preserves
the driver's `mongodb::error::BulkWriteError`: `write_errors` maps each
failure back to its **original queued operation index**, and `partial_result`
describes the writes that succeeded. `OxiModError::bulk_write_error()` reaches
that detail without manual downcasting:

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

The full driver error also remains available through
`std::error::Error::source`.

## One model per batch

A `BulkWrite<M>` batch targets one model and therefore one collection.
MongoDB's `bulkWrite` can span namespaces; for cross-namespace batches — or
driver capabilities outside the typed surface, such as raw update pipelines —
use `mongodb::Client::bulk_write` directly. That escape hatch bypasses OxiMod
validation and typed field-name checking for those operations.

## Related material

* Runnable workflow: `bulk_write` in
  [Runnable Examples](../reference/examples.md) (requires MongoDB Server
  8.0+).
