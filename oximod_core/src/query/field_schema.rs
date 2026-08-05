//! Support for generated typed-field schemas.
//!
//! [`FieldSchema`] connects a model to the generated field structure used by
//! OxiMod's typed query and update APIs.
//!
//! Collection-backed models and models declared with `#[model(embedded)]`
//! implement this trait automatically through `#[derive(Model)]`. Applications
//! normally do not implement or reference it directly.

/// A model whose fields can be represented as typed MongoDB paths.
///
/// OxiMod's `Model` derive generates an associated field structure containing
/// typed [`Field`](crate::query::Field) values. Each generated field stores its
/// complete serialized MongoDB path, including any supplied parent, positional,
/// or array-filter prefix.
///
/// Collection-backed models use an empty prefix at the root of a query.
/// Embedded models and embedded-array operations provide the prefix of the
/// containing MongoDB path.
///
/// # Example
///
/// ```ignore
/// use oximod::{
///     Model,
///     Queryable,
/// };
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     Model,
///     Serialize,
///     Deserialize,
///     Debug,
/// )]
/// #[model(embedded)]
/// struct Address {
///     city: String,
///     active: bool,
/// }
///
/// #[derive(
///     Model,
///     Serialize,
///     Deserialize,
///     Debug,
/// )]
/// #[db("app")]
/// #[collection("users")]
/// struct User {
///     name: String,
///     address: Address,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let users = User::query()
///     .filter(|user| {
///         user.address.nested(|address| {
///             address.city.eq("City1")
///                 & address.active.eq(true)
///         })
///     })
///     .all()
///     .await?;
///
/// # let _ = users;
/// # Ok(())
/// # }
/// ```
///
/// The generated field paths preserve nesting. In this example, the `city`
/// field is represented internally as `"address.city"`.
///
/// Optional embedded models are supported through the implementation for
/// `Option<T>`.
#[doc(hidden)]
pub trait FieldSchema {
    /// The generated typed-field structure associated with this model.
    ///
    /// Each field in the structure contains its complete serialized MongoDB
    /// path, including any supplied prefix.
    type Fields;

    /// Creates the generated typed-field structure beneath `prefix`.
    ///
    /// An empty prefix creates root-level paths. A nonempty prefix creates
    /// nested or positional paths using MongoDB dot notation.
    ///
    /// Generated query code calls this method when a model is used at the root
    /// of a query, nested inside another model, or inside an embedded-model
    /// array operation.
    #[doc(hidden)]
    fn fields_with_prefix(prefix: &str) -> Self::Fields;
}

/// Treats an optional model as having the same generated typed fields as its
/// contained model.
///
/// This allows `Field<Option<T>>` to use the same nested-query and
/// nested-update operations as `Field<T>`:
///
/// ```ignore
/// User::query()
///     .filter(|user| {
///         user.address.nested(|address| {
///             address.city.eq("City1")
///         })
///     })
/// ```
///
/// Whether the stored model is required or optional does not change its
/// generated MongoDB paths.
impl<T> FieldSchema for Option<T>
where
    T: FieldSchema,
{
    type Fields = T::Fields;

    fn fields_with_prefix(prefix: &str) -> Self::Fields {
        T::fields_with_prefix(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::FieldSchema;

    #[derive(Debug, PartialEq, Eq)]
    struct TestFields {
        prefix: String,
    }

    struct Address;

    impl FieldSchema for Address {
        type Fields = TestFields;

        fn fields_with_prefix(prefix: &str) -> Self::Fields {
            TestFields {
                prefix: prefix.to_owned(),
            }
        }
    }

    #[test]
    fn field_schema_receives_field_prefix() {
        assert_eq!(
            Address::fields_with_prefix("address"),
            TestFields {
                prefix: "address".to_owned(),
            },
        );
    }

    #[test]
    fn optional_field_schema_preserves_fields() {
        assert_eq!(
            <Option<Address> as FieldSchema>::fields_with_prefix("profile.address",),
            TestFields {
                prefix: "profile.address".to_owned(),
            },
        );
    }

    #[test]
    fn field_schema_accepts_empty_root_prefix() {
        assert_eq!(
            Address::fields_with_prefix(""),
            TestFields {
                prefix: String::new(),
            },
        );
    }
}
