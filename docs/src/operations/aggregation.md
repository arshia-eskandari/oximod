# Aggregation

Import `Queryable` to call `ModelType::aggregate()`:

```rust
use oximod::Queryable;
```

`aggregate()` starts an ordered aggregation pipeline for the model's
collection. Every stage method appends exactly at its call position; OxiMod
never reorders, merges, or rewrites the pipeline, because MongoDB evaluates
aggregation stages strictly in order.

```rust
let adults = User::aggregate()
    .match_(|user| user.active.eq(true) & user.age.gte(18))
    .sort_by(|user| user.age.desc().then(user.name.asc()))
    .limit(20)
    .all()
    .await?;
```

## Typed stages

* `match_(...)` appends a `$match` stage built from the same typed expressions
  as `query().filter(...)`. Repeated calls append repeated stages in call
  order; they are never merged.
* `sort_by(...)` appends a `$sort` stage. One stage carries several keys by
  chaining `then`: `user.role.asc().then(user.age.desc())`. Calling `sort_by`
  again appends another stage rather than replacing the first, because sort
  position is pipeline semantics.
* `skip(n)` and `limit(n)` append `$skip` and `$limit` stages. Unlike `Query`,
  these are ordered stages, not query-wide modifiers.
* `text(...)` appends a `$match` stage containing `$text`. MongoDB requires it
  to be the pipeline's first stage and the collection needs a text index.

Typed stages use serialized field names, following supported Serde renames,
exactly like typed queries. MongoDB forbids some query operators inside an
aggregation `$match`, notably `$near` and `$nearSphere`; geospatial distance
pipelines use a raw `$geoNear` stage as the first pipeline stage instead.

## Raw stages

The rest of MongoDB's aggregation language — `$group`, `$project`, `$geoNear`,
`$lookup`, `$unwind`, `$facet`, and everything else — is reached through raw
stages:

```rust
use mongodb::bson::doc;

let summaries = User::aggregate()
    .match_(|user| user.active.eq(true))
    .raw_stage_with(|user| {
        doc! {
            "$group": {
                "_id": format!("${}", user.role.name()),
                "count": { "$sum": 1 },
            },
        }
    })
    .with_type::<RoleSummary>()
    .all()
    .await?;
```

* `raw_stage(doc! { ... })` appends one stage document unchanged.
* `raw_stage_with(|fields| doc! { ... })` is the same, but the closure
  receives the generated fields, so source field references built from
  `fields.<name>.name()` stay compiler-linked — a renamed or removed model
  field becomes a compile error instead of a silently broken pipeline.
* `raw_pipeline([...])` appends several stage documents in order.

Raw stage BSON is not operator-checked or path-checked by OxiMod, and
MongoDB's server rules still apply: `$geoNear` must be the first stage, and
the write stages `$out` and `$merge` are restricted inside transactions. A
server rejection surfaces as `OxiModError::Aggregation`.

## Output types

The source model and the output type are distinct concepts: a pipeline may
preserve the model shape, extend it, or replace it entirely.

* `$match`, `$sort`, `$skip`, and `$limit` preserve the model shape, so
  results deserialize as the model by default.
* `$addFields` may preserve the model shape if the extra fields are ignored
  during deserialization; use a dedicated output type when the added data
  matters.
* `$group` and `$project` typically replace the shape entirely and require
  `with_type`.

```rust
#[derive(Debug, serde::Deserialize)]
struct RoleSummary {
    #[serde(rename = "_id")]
    role: String,
    count: i64,
}
```

`with_type::<R>()` changes only how output deserializes; it adds no stage.
OxiMod cannot verify at compile time that an output struct matches what the
pipeline produces — after a shape-changing raw stage, that synchronization is
the caller's responsibility. If an output document does not deserialize as the
selected type, execution fails with `OxiModError::Serialization` rather than
silently dropping or defaulting values, and one undecodable document fails the
whole `all()` call.

After a shape-changing raw stage, use typed source-field stages only if those
source fields still exist with the same meaning; otherwise continue with raw
stages and set the final output type explicitly.

## Execution

```rust
let users = User::aggregate().match_(|user| user.active.eq(true)).all().await?;
let first = User::aggregate().sort_by(|user| user.age.desc()).limit(1).first().await?;

let users = User::aggregate().match_(|user| user.active.eq(true)).all_from(&client).await?;
let users = User::aggregate().match_(|user| user.active.eq(true)).all_with_session(&mut session).await?;
```

* `all()` / `first()` execute through the global client.
* `all_from(&client)` / `first_from(&client)` execute through an explicit
  `mongodb::Client`, so aggregation does not require global initialization.
* `all_with_session(&mut session)` / `first_with_session(&mut session)`
  resolve the collection from the session's own client and advance the cursor
  with the same session, so inside a transaction the pipeline sees the
  transaction's own uncommitted writes. Session participation is always
  explicit.

`first()` runs the pipeline exactly as built and reads at most one result from
the cursor; it does not append a server-side `$limit: 1`, because rewriting
the pipeline could change raw-stage behavior. Add an explicit `.limit(1)`
stage to stop the server early.

There is no streaming terminal; `all()` materializes results the same way
`query().all()` does. For cursor streaming or database-level aggregation, use
the [raw collection escape hatch](../advanced/driver-access.md).

## Driver options

```rust
use mongodb::options::AggregateOptions;

let options = AggregateOptions::builder().allow_disk_use(true).build();

let users = User::aggregate()
    .match_(|user| user.active.eq(true))
    .with_options(options)
    .all()
    .await?;
```

`with_options` passes the driver's `AggregateOptions` through unchanged —
batch size, max time, collation, comment, hint, `let` variables, and
read/write concerns remain driver and server behavior — and applies
identically to global, explicit-client, and session execution.

## Aggregation errors

* connectivity failure during aggregation → `OxiModError::Connection`;
* output BSON decode failure → `OxiModError::Serialization`;
* every other aggregation server/driver operational failure, such as a
  rejected pipeline → `OxiModError::Aggregation`, with the original
  `mongodb::error::Error` preserved as `source()`.

## Related material

* Runnable workflow: `aggregate_usage` in
  [Runnable Examples](../reference/examples.md).
