# Choosing the Right API

| Goal                                               | Recommended API                      |
| -------------------------------------------------- | ------------------------------------ |
| Construct and validate a model                     | Generated builder and `validate()`   |
| Save or work by `_id`                              | `Model` methods                      |
| Type-safe filters, sorting, pagination, and writes | `Queryable`                          |
| Explicit-client persistence                        | `_from` model methods                |
| Raw filters with typed model results               | `Collection<Model>`                  |
| Dynamic BSON documents                             | `Collection<Document>`               |
| Batch mixed writes into one bulk execution         | `ModelType::bulk_write()` builder    |
| Cross-namespace or raw-pipeline bulk writes        | `mongodb::Client::bulk_write`        |
| Aggregation pipelines                              | `Queryable::aggregate()` builder     |
| Streaming or database-level aggregation            | Direct MongoDB collection access     |
| Compound, partial/filtered, or unsupported index options | MongoDB driver index API       |
| Transactional model and typed-query operations     | `_with_session` methods              |
| Advanced session and driver features               | Direct MongoDB collection/client API |

OxiMod is designed so these approaches can coexist in the same application.
