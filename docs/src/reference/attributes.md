# Attributes

## Struct-level attributes

| Attribute                             | Applies to        | Description                                                        |
| ------------------------------------- | ----------------- | ------------------------------------------------------------------ |
| `#[model(embedded)]`                  | Embedded models   | Marks a model as embedded instead of collection-backed.            |
| `#[db("name")]`                       | Collection models | Required database name.                                            |
| `#[collection("name")]`               | Collection models | Required collection name.                                          |
| `#[document_id_setter_ident("name")]` | Collection models | Renames the generated `_id` setter.                                |
| `#[hooks]`                            | Collection models | Generates lifecycle-hook calls.                                    |
| `#[index_max_retries(N)]`             | Collection models | Accepted and stored; not currently enforced as a hard retry limit. |
| `#[index_max_init_seconds(N)]`        | Collection models | Accepted and stored; not currently enforced as a hard timeout.     |

## Field-level attributes

| Attribute                | Description                                                          |
| ------------------------ | -------------------------------------------------------------------- |
| `#[default(expression)]` | Replaces the field's `Default::default()` initialization expression. |
| `#[validate(...)]`       | Adds built-in or custom validation rules.                            |
| `#[index(...)]`          | Adds a generated single-field MongoDB index on a collection model.   |

Serde field and container renames are used when generating typed query paths.

## Where each attribute is documented

* `#[model(embedded)]`, `#[db]`, `#[collection]` —
  [Collection and Embedded Models](../models/collection-and-embedded.md);
* `#[default]`, `#[document_id_setter_ident]` —
  [Builders, Defaults, and IDs](../models/builders-defaults-and-ids.md);
* `#[validate(...)]` — [Validation](../models/validation.md);
* `#[index(...)]` — [Declaring Indexes](../indexes/declarations.md);
* `#[index_max_retries]`, `#[index_max_init_seconds]` —
  [Establishment and Lifecycle](../indexes/lifecycle.md);
* `#[hooks]` — [Lifecycle Hooks](../advanced/hooks.md).
