# Validation

Both collection and embedded models receive an inherent `validate()` method.
OxiMod evaluates all configured rules and returns the failures together rather
than stopping at the first invalid field.

```rust
let user = User::new()
    .email("not-an-email")
    .name("ab")
    .age(-1);

if let Err(error) = user.validate() {
    if let Some(errors) = error.validation_errors() {
        for failure in errors {
            println!("{}: {}", failure.field, failure.message);
        }
    }
}
```

Validation also runs automatically before:

* `save()`;
* `save_mut()`;
* `save_from()`;
* `save_from_mut()`;
* `save_with_session()`;
* `save_mut_with_session()`;
* bulk-write `insert`, `insert_many`, and `replace_one` execution (as the
  whole-model preflight).

For hook-enabled saves, the corresponding pre-save hook runs before
validation, allowing mutable hooks to normalize or populate values before they
are checked.

## Built-in validators

### Optional values

| Validator  | Description                    |
| ---------- | ------------------------------ |
| `required` | Rejects `None` on `Option<T>`. |

Other validators on `Option<T>` run only when the option contains a value.

### Length

| Validator        | Description                                                                |
| ---------------- | -------------------------------------------------------------------------- |
| `min_length = N` | Requires a length of at least `N`.                                         |
| `max_length = N` | Requires a length of at most `N`.                                          |
| `non_empty`      | Rejects empty collections and empty or whitespace-only string-like values. |

Length validation supports string-like values, arrays, sequential collections,
sets, and maps where supported by the derive.

### Strings

| Validator             | Description                           |
| --------------------- | ------------------------------------- |
| `email`               | Requires a valid basic email shape.   |
| `pattern = "..."`     | Matches a regular expression.         |
| `starts_with = "..."` | Requires the prefix.                  |
| `ends_with = "..."`   | Requires the suffix.                  |
| `includes = "..."`    | Requires the substring.               |
| `alphanumeric`        | Allows ASCII letters and digits only. |

String validation supports `String`, `str`-like values, and supported
`Cow<str>` forms.

### Numbers

| Validator       | Description                                     |
| --------------- | ----------------------------------------------- |
| `min = N`       | Inclusive minimum by default.                   |
| `max = N`       | Inclusive maximum by default.                   |
| `min_exclusive` | Changes `min` to a strict lower bound.          |
| `max_exclusive` | Changes `max` to a strict upper bound.          |
| `positive`      | Requires a value greater than zero.             |
| `negative`      | Requires a value less than zero.                |
| `non_negative`  | Requires a value greater than or equal to zero. |
| `non_positive`  | Requires a value less than or equal to zero.    |

### Integers

| Validator         | Description                         |
| ----------------- | ----------------------------------- |
| `multiple_of = N` | Requires exact divisibility by `N`. |

### Nested embedded models

| Validator | Description                                                             |
| --------- | ----------------------------------------------------------------------- |
| `nested`  | Recursively validates embedded OxiMod models reached through the field. |

`nested` is valid only on fields whose type resolves through the supported
containers to a `#[model(embedded)]` model; other target types are rejected at
compile time. See [Nested validation](#nested-validation-of-embedded-models).

## Custom validators

A custom validator is an ordinary function referenced by path:

```rust
fn validate_username(value: &String) -> Result<(), &'static str> {
    if value.eq_ignore_ascii_case("admin") {
        return Err("username is reserved");
    }

    Ok(())
}
```

```rust
#[validate(custom(crate::validate_username))]
username: String,
```

The function receives `&T` and may return any error implementing `ToString`.
For `Option<T>`, it receives `&T` and runs only for `Some(value)`.

## Nested validation of embedded models

Validation descends into an embedded model only where the containing field
explicitly opts in with `#[validate(nested)]`. A field without the attribute
keeps the no-descent behavior: the parent validates and saves without
evaluating the embedded value's own rules, and the embedded type's
`validate()` works normally when called directly.

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct Address {
    #[validate(non_empty)]
    city: String,

    #[validate(pattern = r"^[0-9]{5}$")]
    postal_code: String,
}

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(nested)]
    address: Address,

    #[validate(nested)]
    previous_addresses: Vec<Address>,
}
```

One `nested` marker descends recursively through supported container wrappers
until it reaches the embedded model:

* a bare embedded field;
* `Option<Embedded>`;
* `Vec<Embedded>`;
* `HashMap<String, Embedded>`;
* recursive combinations such as `Vec<Option<Embedded>>`,
  `Option<Vec<Embedded>>`, and `HashMap<String, Vec<Option<Embedded>>>`.

Each model-to-model containment edge remains opt-in: when an embedded model
contains another embedded model, the inner containing field must also carry
`#[validate(nested)]` for validation to descend further. Marking a field whose
type does not resolve to an embedded model (such as a scalar or a
collection-backed model) fails at compile time.

Container semantics compose with the existing rules:

* `None` produces no nested errors; add `required` to reject absence;
* empty vectors and maps produce no nested errors; add `non_empty` to reject
  emptiness;
* the field's own rules and its descendants' rules are all evaluated and
  aggregated together with the rest of the model's failures.

### Nested error paths

Descendant failures keep their exact messages and report path-aware `field`
values:

```text
address.postal_code
previous_addresses[1].city
addresses["billing"].postal_code
orders[2].shipping_address.postal_code
```

Paths use Rust model field names, matching top-level validation attribution;
Serde/BSON renames affect stored documents and typed queries, not validation
paths. Map keys are quoted with `"` and `\` escaped, so keys containing dots,
spaces, brackets, or quotes cannot masquerade as path segments. Error order is
deterministic: parent declaration order, depth-first into each nested model's
own declaration order, vector elements in ascending index order, and map
entries in lexicographically sorted key order. The path strings are
deterministic and human-readable; they are not a stable machine-parseable
schema.

### Where nested validation applies

Because `#[validate(nested)]` changes what the model's own validation
evaluates, it applies automatically anywhere whole-model validation already
runs: `validate()`, every save form (including session-aware saves), and the
bulk-write insert/replace preflight. Typed and raw update expressions remain
non-validating (see [Validation and updates](#validation-and-updates)).

### Custom delegation remains possible

The previous custom-validator delegation pattern remains valid — for example
to combine descent with cross-field checks — but is no longer required merely
to descend into an embedded value:

```rust
fn validate_address(address: &Address) -> Result<(), String> {
    address.validate().map_err(|error| error.to_string())
}
```

```rust
#[validate(custom(validate_address))]
address: Address,
```

Custom delegation reports the child's failures as one error attributed to the
containing field, while `nested` reports each descendant failure under its own
path. Pre-save hook guards likewise remain possible but are no longer required
for descent; lifecycle hooks are unchanged by nested validation.

## Validation and updates

Validation is **not** automatically applied to:

* raw MongoDB update documents;
* typed `update_one()` or `update_all()` expressions;
* direct collection operations.

These operations modify stored documents through MongoDB. Application code
remains responsible for choosing values that preserve model invariants.

## Related material

* Structured validation failures and `validation_errors()` are covered in
  [Errors and Behavioral Boundaries](../advanced/errors-and-boundaries.md).
* Runnable workflows: `validate_usage`, `custom_validate`,
  `nested_validation`, and `validate_extract_errors` in
  [Runnable Examples](../reference/examples.md).
