# Builders, Defaults, and IDs

Every derived model receives:

* `ModelType::new()`;
* `Default::default()`;
* a fluent setter for each field.

Ordinary setters accept values through `Into<T>`. Setters for `Option<T>`
accept a value convertible into `T` and store it as `Some(...)`.

```rust
let user = User::new()
    .name("User1")
    .email("user1@example.com")
    .age(30)
    .active(true);
```

## Construction is not typestate

OxiMod does not require every setter to be called. During `new()`:

* fields with `#[default(...)]` use that expression;
* all other fields use `Default::default()`.

This means a required application field such as `String` begins as an empty
string unless it has a configured default or is set through the builder. Use
[validation](validation.md) to enforce domain requirements.

## Defaults

`#[default(...)]` accepts an ordinary Rust expression convertible into the
field type:

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[model(embedded)]
struct Preferences {
    #[default(String::from("en-CA"))]
    language: String,

    #[default(true)]
    notifications: bool,

    #[default(25_u32)]
    page_size: u32,

    nickname: Option<String>,
}
```

```rust
let preferences = Preferences::new().nickname("User1");

assert_eq!(preferences.language, "en-CA");
assert!(preferences.notifications);
assert_eq!(preferences.page_size, 25);
assert_eq!(preferences.nickname.as_deref(), Some("User1"));
```

Defaults are evaluated during construction and remain overridable through
generated setters. Numeric literals should be suffixed when Rust's default
literal type cannot convert into the field type.

## Defaults and schema evolution

Defaults are construction-time only; they are never applied when reading
documents from MongoDB. A stored document that lacks the field entirely fails
to deserialize regardless of `#[default(...)]`. When adding a field to a model
whose collection already contains documents, give the field a read-side
default with `#[serde(default = "path")]`. Avoid bare `#[serde(default)]` as a
schema-evolution strategy: it substitutes the field type's
`Default::default()` value — not your configured `#[default(...)]` — and a
later save writes that substituted value back to the database.

## MongoDB `_id` setter

For a collection field named `_id`, the generated builder setter is `id()` by
default:

```rust
let user = User::new()
    .id(ObjectId::new())
    .name("User1");
```

Rename it when needed:

```rust
#[document_id_setter_ident("with_id")]
```

```rust
let user = User::new()
    .with_id(ObjectId::new())
    .name("User1");
```
