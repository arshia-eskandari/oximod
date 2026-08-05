//! Embedded-model field paths and embedded-array operations.
//!
//! This module provides typed query and update methods for model fields whose
//! values implement [`FieldSchema`](crate::query::FieldSchema).
//!
//! Models declared with `#[model(embedded)]` receive a generated field schema.
//! `Option<T>` forwards the schema of `T`, so required and optional embedded
//! fields use the same nested API.

use mongodb::bson::Bson;

use super::Field;
use crate::query::FieldSchema;
use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::update_expression::UpdateExpression;

impl<T> Field<Vec<T>>
where
    T: FieldSchema,
{
    /// Matches an array containing an embedded model that satisfies the
    /// supplied typed conditions.
    ///
    /// The generated fields are relative to the array element, so their paths
    /// do not include the array field name inside `$elemMatch`.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.addresses.elem_match_nested(
    ///             |address| {
    ///                 address.city.eq("City1")
    ///                     & address.active.eq(true)
    ///             },
    ///         )
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn elem_match_nested<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&T::Fields) -> Expression,
    {
        let fields = T::fields_with_prefix("");
        let expression = build(&fields);

        Expression::comparison(
            self.name(),
            ComparisonOperator::ElemMatch,
            Bson::Document(expression.into_document()),
        )
    }

    /// Provides typed fields targeting the first array element matched by the
    /// query through MongoDB's positional `$` operator.
    ///
    /// The array field must participate in the query filter so MongoDB can
    /// determine which element the `$` placeholder represents.
    ///
    /// ```ignore
    /// User::query()
    ///     .filter(|user| {
    ///         user.addresses.elem_match_nested(
    ///             |address| {
    ///                 address.city.eq("City1")
    ///             },
    ///         )
    ///     })
    ///     .update_one(|user| {
    ///         user.addresses.positional(
    ///             |address| {
    ///                 address.active.set(true)
    ///             },
    ///         )
    ///     })
    ///     .await?;
    /// ```
    pub fn positional<F>(&self, build: F) -> UpdateExpression
    where
        F: FnOnce(&T::Fields) -> UpdateExpression,
    {
        let prefix = format!("{}.$", self.name());
        let fields = T::fields_with_prefix(&prefix);

        build(&fields)
    }

    /// Provides typed fields targeting array elements identified by a filtered
    /// positional placeholder such as `$[address]`.
    ///
    /// A corresponding array filter with the same identifier must be supplied
    /// through [`crate::query::Query::array_filter`].
    pub fn filtered<F>(&self, identifier: impl AsRef<str>, build: F) -> UpdateExpression
    where
        F: FnOnce(&T::Fields) -> UpdateExpression,
    {
        let prefix = format!("{}.$[{}]", self.name(), identifier.as_ref(),);
        let fields = T::fields_with_prefix(&prefix);

        build(&fields)
    }

    /// Builds a typed MongoDB array-filter condition.
    ///
    /// The identifier becomes the root prefix of each generated filter path
    /// and must match the identifier passed to [`Field::filtered`].
    ///
    /// ```ignore
    /// User::query()
    ///     .array_filter(|user| {
    ///         user.addresses.array_filter(
    ///             "address",
    ///             |address| {
    ///                 address.active.eq(false)
    ///             },
    ///         )
    ///     })
    ///     .update_one(|user| {
    ///         user.addresses.filtered(
    ///             "address",
    ///             |address| {
    ///                 address.active.set(true)
    ///             },
    ///         )
    ///     })
    ///     .await?;
    /// ```
    pub fn array_filter<F>(&self, identifier: impl AsRef<str>, build: F) -> Expression
    where
        F: FnOnce(&T::Fields) -> Expression,
    {
        let fields = T::fields_with_prefix(identifier.as_ref());

        build(&fields)
    }
}

impl<T> Field<T>
where
    T: FieldSchema,
{
    /// Provides the generated typed fields of this nested model.
    ///
    /// The generated fields preserve the complete serialized MongoDB path.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.address.nested(|address| {
    ///             address.city.eq("City1")
    ///         })
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// This method can return a query expression, sort expression, update
    /// expression, or another value depending on what the closure builds.
    ///
    /// Required and optional nested models use the same API because
    /// `Option<T>` forwards its field schema to `T`.
    pub fn nested<R, F>(&self, build: F) -> R
    where
        F: FnOnce(&T::Fields) -> R,
    {
        let fields = T::fields_with_prefix(self.name());

        build(&fields)
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{Field, FieldSchema};
    use mongodb::bson::doc;

    struct Address;

    struct AddressFields {
        city: Field<String>,
        active: Field<bool>,
    }

    impl FieldSchema for Address {
        type Fields = AddressFields;

        fn fields_with_prefix(prefix: &str) -> Self::Fields {
            AddressFields {
                city: Field::from_prefixed(prefix, "city"),
                active: Field::from_prefixed(prefix, "active"),
            }
        }
    }

    #[test]
    fn nested_field_prefixes_embedded_field_paths() {
        let address = Field::<Address>::new("address");

        let expression =
            address.nested(|address| address.city.eq("City1") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "address.city": "City1",
                    },
                    {
                        "address.active": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn optional_nested_field_prefixes_embedded_paths() {
        let address = Field::<Option<Address>>::new("address");

        let expression =
            address.nested(|address| address.city.eq("City1") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "address.city": "City1",
                    },
                    {
                        "address.active": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn embedded_model_array_builds_elem_match_expression() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let expression = addresses
            .elem_match_nested(|address| address.city.eq("City1") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "addresses": {
                    "$elemMatch": {
                        "$and": [
                            {
                                "city": "City1",
                            },
                            {
                                "active": true,
                            },
                        ],
                    },
                },
            }
        );
    }

    #[test]
    fn embedded_array_positional_builds_prefixed_update_path() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.positional(|address| address.active.set(true));

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "addresses.$.active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_filtered_builds_identifier_update_path() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.filtered("address", |address| address.active.set(true));

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "addresses.$[address].active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_filtered_combines_multiple_update_paths() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.filtered("address", |address| {
            address.city.set("City2") & address.active.set(true)
        });

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "addresses.$[address].city": "City2",
                    "addresses.$[address].active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_builds_identifier_filter_expression() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let expression = addresses.array_filter("address", |address| address.city.eq("City1"));

        assert_eq!(
            expression.into_document(),
            doc! {
                "address.city": "City1",
            }
        );
    }
}
