# Typed Queries

Import `Queryable` to call `ModelType::query()`:

```rust
use oximod::Queryable;
```

The derive generates a typed field structure for each collection model. Every
field exposes only operations supported by its Rust type, so incompatible code
fails to compile. For example:

* regex methods are unavailable on integer fields;
* ordered comparisons are unavailable on booleans;
* `unset()` is unavailable on required fields;
* array operations are unavailable on scalar fields;
* nested operations require embedded-model field metadata.

Typed queries currently execute through the global `OxiClient`; see the
[typed-query limitation](persistence-and-clients.md#typed-query-limitation)
for explicit-client workflows.

## Filters and logical expressions

```rust
let users = User::query()
    .filter(|user| {
        user.active.eq(true)
            & user.age.gte(18)
            & (
                user.name.eq("User1")
                    | user.name.eq("User2")
            )
    })
    .all()
    .await?;
```

Use:

* `&` for logical AND;
* `|` for logical OR;
* `not(...)` for field-level negation.

Rust does not allow overloading `&&` or `||`. Repeated `filter()` calls are
also combined with AND:

```rust
let users = User::query()
    .filter(|user| user.active.eq(true))
    .filter(|user| user.age.gte(18))
    .all()
    .await?;
```

## Query-operation families

Depending on the field type, generated fields support families such as:

* equality: `eq`, `ne`;
* membership: `in_values`, `not_in_values`;
* ordered comparisons: `gt`, `gte`, `lt`, `lte`;
* field presence: `exists`, `not_exists`;
* optional null checks such as `is_null`;
* BSON type checks;
* regex and escaped string helpers;
* modulo and integer bitwise predicates;
* arrays and `$elemMatch`;
* embedded-model paths;
* GeoJSON geospatial predicates.

Ordered comparisons, numeric updates, and modulo are also available on
`Option<T>` fields whose inner type supports them. The operand is always a
value of the inner type `T` — for example `expires_at.gt(deadline)` on an
`Option<DateTime>` field, or `login_count.inc(1)` on an `Option<i32>` field;
`Some(...)` and `None` operands do not compile. Documents storing BSON null
and documents missing the field follow MongoDB's normal query semantics: an
ordered comparison against an inner value does not match them.

## Serde renames and typed paths

The Rust field name does not have to match the stored MongoDB field name.
Generated paths follow supported Serde renames:

```rust
#[derive(Debug, Serialize, Deserialize, Model)]
#[serde(rename_all = "camelCase")]
#[db("app")]
#[collection("work_items")]
struct WorkItem {
    team_name: String,
}
```

```rust
WorkItem::query()
    .filter(|item| item.team_name.eq("Team1"))
    .all()
    .await?;
```

The generated query targets `teamName` in MongoDB.

`#[serde(alias = "...")]` is a read-side compatibility tool, not a rename
migration. Typed query paths always use a field's primary serialized name, so
documents still stored under a legacy key remain readable through the alias
but are silently missed by typed filters on that field. During a field rename,
migrate the persisted documents — or match both spellings with a raw `$or`
filter through the document collection — before relying on typed queries
against the renamed field.

## Sorting

```rust
let users = User::query()
    .sort_by(|user| user.age.desc())
    .then_sort_by(|user| user.name.asc())
    .all()
    .await?;
```

* `sort_by()` establishes or replaces the primary sort;
* `then_sort_by()` appends another sort field.

Use deterministic secondary sorting when several documents may share the same
primary value.

## Limits, skipping, and pagination

```rust
let page = User::query()
    .filter(|user| user.active.eq(true))
    .sort_by(|user| user.name.asc())
    .page(2, 25)
    .all()
    .await?;
```

Pagination is one-based. Page `2` with size `25` skips the first `25` matching
documents.

Invalid pagination values and limits that cannot be represented by the MongoDB
driver are returned as typed query errors when the query executes.

A page is read as one result window through `all()`, which fails as a whole if
any document in the window cannot be deserialized into the model — documents
are never silently dropped, and none of the window's documents are returned.
Later pages whose windows contain only valid documents still succeed. To
locate, inspect, or repair documents that no longer match the model, read them
as raw BSON through `get_document_collection()`.

## Read execution semantics

| Method    | Result          | Filter | Sort | Skip | Limit / page |
| --------- | --------------- | -----: | ---: | ---: | -----------: |
| `all()`   | `Vec<Model>`    |    Yes |  Yes |  Yes |          Yes |
| `first()` | `Option<Model>` |    Yes |  Yes |   No |           No |
| `count()` | `u64`           |    Yes |   No |   No |           No |

`count()` uses the filter and configured text search, but ignores
result-ordering and result-window modifiers.

## Arrays

Array fields support typed membership and update operations:

```rust
let users = User::query()
    .filter(|user| {
        user.tags.contains_all(["rust", "mongodb"])
            & user.scores.elem_match(|score| {
                score.gte(60) & score.lte(100)
            })
    })
    .all()
    .await?;
```

Query helpers include:

* element membership;
* `$all`;
* exact `$size`;
* scalar `$elemMatch`.

The scalar `elem_match` overload applies to scalar elements; for arrays of
embedded models use `elem_match_nested` (see
[Embedded documents](#embedded-documents)).

Typed array updates include:

* `$push` and multi-value `$push`;
* `$addToSet` and multi-value `$addToSet`;
* `$pull`;
* first- and last-element `$pop`;
* positional and filtered updates for arrays of embedded models.

Array update operators (`push`, `add_to_set`, `pull`, and whole-array `set`)
require the element type to convert into BSON (`Into<Bson>`). Scalar elements
such as strings and numbers qualify automatically; derived embedded models do
not implement `Into<Bson>` automatically. Implement the conversion once per
embedded type to enable these operators on `Vec<Embedded>` fields:

```rust
use mongodb::bson::{Bson, to_bson};

impl From<Address> for Bson {
    fn from(address: Address) -> Self {
        to_bson(&address).expect("Address serializes to BSON")
    }
}
```

`From` conversions cannot report failure, so the implementation must decide
how to handle a value that fails to serialize (this example panics). Typed
*matching* on embedded arrays needs no conversion — `elem_match_nested` works
without it.

## Embedded documents

```rust
let users = User::query()
    .filter(|user| {
        user.address.nested(|address| {
            address.city.eq("City1")
        })
    })
    .all()
    .await?;
```

Optional embedded models support the same nested field schema when present.
Arrays of embedded models add typed nested `$elemMatch`:

```rust
let users = User::query()
    .filter(|user| {
        user.addresses.elem_match_nested(|address| {
            address.city.eq("City1")
                & address.active.eq(true)
        })
    })
    .all()
    .await?;
```

An `elem_match_nested` filter also supplies the array match that MongoDB's
positional `$` update operator requires, so pair it with `positional(...)`
when updating the first matched element.

Nested fields may also be used for sorting and typed updates where supported.

## String and regex queries

String fields support regular expressions and escaped convenience helpers:

```rust
use oximod::RegexOption;

let users = User::query()
    .filter(|user| {
        user.name.matches_regex_with_options(
            "^user",
            [RegexOption::CaseInsensitive],
        )
    })
    .all()
    .await?;
```

`RegexOption` maps to MongoDB's common regex options:

* case-insensitive;
* multiline;
* dot matches newline;
* ignore pattern whitespace.

Convenience helpers such as prefix, suffix, and contained-text checks escape
their input before constructing the regex.

## Text search

Text search requires an appropriate MongoDB text index:

```rust
#[index(text)]
content: String,
```

Use a string for a basic search:

```rust
let articles = Article::query()
    .text("rust mongodb")
    .all()
    .await?;
```

Use `TextSearch` for additional options:

```rust
use oximod::TextSearch;

let articles = Article::query()
    .text(
        TextSearch::new("\"rust mongodb\" -beginner")
            .language("none")
            .case_sensitive(false)
            .diacritic_sensitive(false),
    )
    .sort_by_text_score()
    .all()
    .await?;
```

MongoDB phrase and excluded-term syntax can be included in the search string.

## Geospatial queries

OxiMod provides typed GeoJSON values:

* `GeoPoint`;
* `GeoPolygon`;
* `NearQuery`.

```rust
use oximod::{GeoPoint, NearQuery};

let places = Place::query()
    .filter(|place| {
        place.location.near(
            NearQuery::new(
                GeoPoint::new(-79.38, 43.65),
            )
            .max_distance(5_000.0),
        )
    })
    .all()
    .await?;
```

GeoJSON coordinates use longitude-latitude order. With a `2dsphere` index,
GeoJSON `$near` distances are expressed in metres.

Typed geospatial predicates include `$near`, `$geoWithin`, and
`$geoIntersects` where supported by the field geometry.

OxiMod serializes the geometry but does not fully validate coordinate ranges,
polygon validity, or distance relationships; MongoDB remains the final
authority for the query.

## Related material

* Writing through the same typed fields:
  [Updates and Deletion](updates-and-deletion.md).
* Runnable workflows: `typed_query` and `query` in
  [Runnable Examples](../reference/examples.md).
