# OxiMod Macros

`oximod_macros` contains the procedural-macro implementation used by
[OxiMod](../README.md), a schema-aware MongoDB modeling library for Rust.

Most applications should depend on the main `oximod` crate rather than adding
`oximod_macros` directly. The main crate re-exports both the `Model` derive
macro and the runtime `Model` trait, so users normally need only:

```rust
use oximod::Model;
```

## `Model` derive

Collection-backed models use database and collection attributes:

```rust
use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize)]
#[db("app")]
#[collection("users")]
struct User {
    _id: Option<ObjectId>,
    name: String,
}
```

Embedded models use the same derive with `#[model(embedded)]`:

```rust
use oximod::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize)]
#[model(embedded)]
struct Address {
    city: String,
}
```

Depending on the model kind and its attributes, the derive generates support
for:

* fluent builder setters and field defaults;
* aggregated validation and an inherent `validate()` method;
* typed field schemas for nested queries;
* collection persistence and typed queries;
* MongoDB index initialization;
* optional lifecycle hooks.

Collection persistence, collection indexes, hooks, and `Queryable` are
generated only for collection-backed models.

## Internal structure

The macro implementation is divided by responsibility:

* `default`: builder-setter generation;
* `helpers`: model attribute and field orchestration;
* `parsers`: derive-attribute parsing and diagnostics;
* `validate`: validation parsing and token generation;
* `index`: MongoDB index representation and token generation;
* `model_macro`: validation, persistence, and hook implementations;
* `query`: typed field-schema and `Queryable` generation.

These modules are implementation details of OxiMod and are not part of its
public runtime API.

## Development

Run the focused crate checks with:

```bash
cargo fmt -p oximod_macros \
&& cargo test -p oximod_macros \
&& cargo check -p oximod_macros \
&& cargo clippy -p oximod_macros --all-targets --all-features -- -D warnings
```

## License

This project is licensed under the [MIT License](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in OxiMod by you shall be licensed as MIT, without any additional
terms or conditions.

