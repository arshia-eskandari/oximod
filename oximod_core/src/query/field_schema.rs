//! Support for generated typed field schemas.
//!
//! [`FieldSchema`] connects an OxiMod model to the generated field structure
//! used by the typed query and update APIs.
//!
//! Both collection-backed models and models marked with `#[model(embedded)]`
//! implement this trait automatically through `#[derive(Model)]`.
//!
//! Applications generally do not implement this trait manually.

/// A model whose fields can be represented as typed MongoDB paths.
///
/// OxiMod's `Model` derive generates an associated field structure containing
/// typed [`Field`](crate::query::Field) values. Each generated field stores its
/// complete MongoDB path, including any parent prefixes.
///
/// Collection-backed models use an empty prefix when constructing root-level
/// fields. Embedded models use the path of their containing field.
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
/// The generated field paths preserve nesting. In the example above, the
/// `city` field is represented internally as `"address.city"`.
///
/// Optional embedded models are supported automatically through the
/// implementation for `Option<T>`.
#[doc(hidden)]
pub trait FieldSchema {
    /// The generated typed-field structure associated with this model.
    ///
    /// Each field in this structure contains its complete MongoDB path,
    /// including any parent prefixes.
    type Fields;

    /// Creates the generated typed-field structure using `prefix` as the
    /// containing MongoDB document path.
    ///
    /// An empty prefix creates root-level field paths. A nonempty prefix
    /// creates nested paths using MongoDB dot notation.
    ///
    /// This method is called by generated query code when a model is used at
    /// the root of a query or nested inside another model.
    #[doc(hidden)]
    fn fields_with_prefix(prefix: &str) -> Self::Fields;
}

/// Treats an optional model as having the same generated typed fields as its
/// contained model.
///
/// This enables `Field<Option<T>>` to use the same nested-query and nested-update
/// operations as `Field<T>`:
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
/// Whether the stored value is required or optional does not change its
/// generated MongoDB field paths.
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
