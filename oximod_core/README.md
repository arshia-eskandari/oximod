# OxiMod Core

Core runtime library for [OxiMod](https://github.com/arshia-eskandari/oximod), a schema-aware MongoDB modeling library for Rust.

`oximod_core` provides the shared runtime components used by the main `oximod` crate and its derive macros, including:

* collection-backed and embedded model infrastructure,
* MongoDB client management,
* lifecycle hooks,
* model validation errors,
* typed filters, sorting, pagination, updates, and deletions,
* typed aggregation pipeline construction and execution,
* typed bulk-write batch construction and execution,
* index drift inspection and create-only index reconciliation,
* nested-document and array operations,
* text-search and GeoJSON query support,
* internal asynchronous initialization helpers.

Most applications should depend on the main `oximod` crate rather than using `oximod_core` directly. The main crate exposes the supported public API together with the `Model` derive macro.

## License

This project is licensed under the [MIT License](https://github.com/arshia-eskandari/oximod/blob/main/LICENSE).

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in OxiMod by you is licensed under the MIT License without
additional terms or conditions.

