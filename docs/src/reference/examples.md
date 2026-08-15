# Runnable Examples

The repository includes focused runnable examples under
[`oximod/examples`](https://github.com/arshia-eskandari/oximod/tree/main/oximod/examples).

| Example | Demonstrates |
| --- | --- |
| `basic_usage` | First model, global client, save, and simple reads |
| `default_usage` | Field defaults during construction |
| `by_id` | `find_by_id`, `update_by_id`, and `delete_by_id` workflows |
| `query` | Raw BSON filters through the model's collection |
| `typed_query` | Typed filters, sorting, and pagination |
| `update` | Typed single- and multi-document updates |
| `update_with_client` | Explicit-client persistence with `_from` methods |
| `delete` | Typed single- and multi-document deletion |
| `aggregate_usage` | The aggregation builder with typed and raw stages |
| `bulk_write` | Model-scoped bulk writes (requires MongoDB Server 8.0+) |
| `session_transactions` | Session-aware operations in a transaction (requires a replica set) |
| `validate_usage` | Built-in validation rules |
| `custom_validate` | Custom validator functions |
| `nested_validation` | Opt-in nested validation of embedded models |
| `validate_extract_errors` | Inspecting structured validation failures |
| `hook_usage` | Lifecycle hooks |
| `index_reconciliation` | Index drift inspection and create-only reconciliation |

Run an example with:

```bash
cargo run -p oximod --example typed_query
```

MongoDB-backed examples read `MONGODB_URI` from the environment or a `.env`
file.
