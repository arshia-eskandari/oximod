//! Support for typed queries on embedded documents.
//!
//! [`EmbeddedDocument`] connects an embedded Rust type to the generated field
//! structure used by OxiMod's typed query and update APIs.
//!
//! Applications normally implement this trait through
//! `#[derive(EmbeddedDocument)]` rather than manually.

/// A type whose fields can be addressed through an embedded MongoDB path.
///
/// OxiMod's `EmbeddedDocument` derive generates an associated field structure
/// containing typed [`Field`](crate::query::Field) values. Each generated field
/// includes the path of its parent document.
///
/// # Example
///
/// ```ignore
/// use oximod::{
///     EmbeddedDocument,
///     Model,
/// };
/// use serde::{
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(
///     EmbeddedDocument,
///     Serialize,
///     Deserialize,
///     Debug,
///     Default,
/// )]
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
/// let users = User::query()
///     .filter(|user| {
///         user.address
///             .field(|address| {
///                 address.city.eq("City1")
///             })
///     })
///     .all()
///     .await?;
/// ```
///
/// The generated field paths preserve nesting. For example, the `city` field
/// above is represented internally as `"address.city"`.
///
/// Optional embedded documents are supported automatically through the
/// implementation for `Option<T>`.
pub trait EmbeddedDocument {
    /// The generated field structure for this embedded document.
    ///
    /// Each field in this structure contains its complete MongoDB path,
    /// including any parent prefixes.
    type Fields;

    /// Creates the generated field structure using `prefix` as the document
    /// path.
    ///
    /// This method is called by generated query code when an embedded document
    /// is nested inside a model or another embedded document.
    #[doc(hidden)]
    fn fields_with_prefix(prefix: &str) -> Self::Fields;
}

/// Treats an optional embedded document as having the same typed fields as its
/// contained value.
///
/// This allows required and optional embedded documents to use the same query
/// syntax:
///
/// ```ignore
/// User::query()
///     .filter(|user| {
///         user.address
///             .field(|address| {
///                 address.city.eq("City1")
///             })
///     })
/// ```
///
/// The presence or absence of the value does not change its generated MongoDB
/// field paths.
impl<T> EmbeddedDocument for Option<T>
where
    T: EmbeddedDocument,
{
    type Fields = T::Fields;

    fn fields_with_prefix(prefix: &str) -> Self::Fields {
        T::fields_with_prefix(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::EmbeddedDocument;

    #[derive(Debug, PartialEq, Eq)]
    struct TestFields {
        prefix: String,
    }

    struct Address;

    impl EmbeddedDocument for Address {
        type Fields = TestFields;

        fn fields_with_prefix(prefix: &str) -> Self::Fields {
            TestFields {
                prefix: prefix.to_owned(),
            }
        }
    }

    #[test]
    fn embedded_document_receives_field_prefix() {
        assert_eq!(
            Address::fields_with_prefix("address",),
            TestFields {
                prefix: "address".to_owned(),
            },
        );
    }

    #[test]
    fn optional_embedded_document_preserves_fields() {
        assert_eq!(
            <Option<Address> as EmbeddedDocument>::fields_with_prefix("profile.address",),
            TestFields {
                prefix: "profile.address".to_owned(),
            },
        );
    }
}
