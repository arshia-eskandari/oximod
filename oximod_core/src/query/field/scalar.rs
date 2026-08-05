//! Scalar field queries and updates.
//!
//! This module provides equality, membership, ordered comparison, strict null
//! checks, and scalar update operations for typed [`Field`] values.

use mongodb::bson::Bson;

use super::{Field, OrderedQueryValue};
use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::update_expression::UpdateExpression;

impl<T> Field<Option<T>> {
    /// Matches documents where this field exists and contains BSON null.
    ///
    /// MongoDB equality against null also matches missing fields. OxiMod adds
    /// an existence condition so this method matches only explicitly stored
    /// null values.
    pub fn is_null(&self) -> Expression {
        Expression::comparison(self.name(), ComparisonOperator::Eq, Bson::Null) & self.exists()
    }

    /// Matches documents where this field exists and is not BSON null.
    ///
    /// Missing fields are excluded from the result.
    pub fn is_not_null(&self) -> Expression {
        Expression::comparison(self.name(), ComparisonOperator::Ne, Bson::Null) & self.exists()
    }

    /// Removes this optional field from the stored MongoDB document.
    ///
    /// ```ignore
    /// User::query()
    ///     .filter(|user| user.name.eq("User1"))
    ///     .update_one(|user| {
    ///         user.nickname.unset()
    ///     })
    ///     .await?;
    /// ```
    pub fn unset(&self) -> UpdateExpression {
        UpdateExpression::unset(self.name())
    }

    /// Renames this optional field to another optional field of the same type.
    ///
    /// MongoDB moves the stored value to the destination path and removes the
    /// source path.
    pub fn rename_to(&self, destination: &Field<Option<T>>) -> UpdateExpression {
        UpdateExpression::rename(self.name(), destination.name())
    }
}

impl<T> Field<T>
where
    T: Into<Bson>,
{
    /// Matches documents where this field equals `value`.
    pub fn eq<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Eq, value)
    }

    /// Matches documents where this field does not equal `value`.
    pub fn ne<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Ne, value)
    }

    /// Matches documents where this field equals any supplied value.
    ///
    /// This produces MongoDB's `$in` operator.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.role.in_values([
    ///             "admin",
    ///             "member",
    ///         ])
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn in_values<I, V>(&self, values: I) -> Expression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        let values = values
            .into_iter()
            .map(|value| {
                let value: T = value.into();
                value.into()
            })
            .collect::<Vec<Bson>>();

        Expression::comparison(self.name(), ComparisonOperator::In, Bson::Array(values))
    }

    /// Matches documents where this field equals none of the supplied values.
    ///
    /// This produces MongoDB's `$nin` operator.
    pub fn not_in_values<I, V>(&self, values: I) -> Expression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        let values = values
            .into_iter()
            .map(|value| {
                let value: T = value.into();
                value.into()
            })
            .collect::<Vec<Bson>>();

        Expression::comparison(self.name(), ComparisonOperator::Nin, Bson::Array(values))
    }

    /// Replaces this field with `value`.
    ///
    /// This produces MongoDB's `$set` update.
    ///
    /// ```ignore
    /// User::query()
    ///     .filter(|user| user.name.eq("User1"))
    ///     .update_one(|user| {
    ///         user.active.set(true)
    ///     })
    ///     .await?;
    /// ```
    pub fn set<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::set(self.name(), value.into())
    }

    fn comparison<V>(&self, operator: ComparisonOperator, value: V) -> Expression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        Expression::comparison(self.name(), operator, value)
    }
}

impl<T> Field<T>
where
    T: OrderedQueryValue + Into<Bson>,
{
    /// Matches documents where this field is greater than `value`.
    pub fn gt<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gt, value)
    }

    /// Matches documents where this field is greater than or equal to
    /// `value`.
    pub fn gte<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gte, value)
    }

    /// Matches documents where this field is less than `value`.
    pub fn lt<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lt, value)
    }

    /// Matches documents where this field is less than or equal to `value`.
    pub fn lte<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lte, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{
        Field,
        expression::{ComparisonOperator, Expression},
    };
    use mongodb::bson::{Bson, doc};

    #[test]
    fn equality_builds_an_expression() {
        let active = Field::<bool>::new("active");

        assert_eq!(
            active.eq(true).into_document(),
            doc! {
                "active": true,
            }
        );
    }

    #[test]
    fn inequality_builds_an_expression() {
        let status = Field::<String>::new("status");

        assert_eq!(
            status.ne("deleted").into_document(),
            doc! {
                "status": {
                    "$ne": "deleted",
                },
            }
        );
    }

    #[test]
    fn string_field_accepts_a_string_slice() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.eq("User1").into_document(),
            doc! {
                "name": "User1",
            }
        );
    }

    #[test]
    fn greater_than_builds_an_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.gt(18).into_document(),
            doc! {
                "age": {
                    "$gt": 18,
                },
            }
        );
    }

    #[test]
    fn greater_than_or_equal_builds_an_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.gte(18).into_document(),
            doc! {
                "age": {
                    "$gte": 18,
                },
            }
        );
    }

    #[test]
    fn less_than_builds_an_expression() {
        let price = Field::<f64>::new("price");

        assert_eq!(
            price.lt(99.99).into_document(),
            doc! {
                "price": {
                    "$lt": 99.99,
                },
            }
        );
    }

    #[test]
    fn less_than_or_equal_builds_an_expression() {
        let price = Field::<i64>::new("price");

        assert_eq!(
            price.lte(100_i64).into_document(),
            doc! {
                "price": {
                    "$lte": 100_i64,
                },
            }
        );
    }

    #[test]
    fn field_builds_in_expression() {
        let role = Field::<String>::new("role");

        assert_eq!(
            role.in_values(["admin", "moderator",]).into_document(),
            doc! {
                "role": {
                    "$in": [
                        "admin",
                        "moderator",
                    ],
                },
            }
        );
    }

    #[test]
    fn field_builds_not_in_expression() {
        let role = Field::<String>::new("role");

        assert_eq!(
            role.not_in_values(["admin", "moderator",]).into_document(),
            doc! {
                "role": {
                    "$nin": [
                        "admin",
                        "moderator",
                    ],
                },
            }
        );
    }

    #[test]
    fn field_builds_strict_null_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.is_null().into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": null,
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn field_builds_strict_not_null_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.is_not_null().into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": {
                            "$ne": null,
                        },
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn field_builds_set_update_expression() {
        let active = Field::<bool>::new("active");

        assert_eq!(
            active.set(true).into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
            }
        );
    }

    #[test]
    fn optional_field_builds_unset_update_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.unset().into_document(),
            doc! {
                "$unset": {
                    "nickname": "",
                },
            }
        );
    }

    #[test]
    fn optional_field_builds_rename_update_expression() {
        let nickname = Field::<Option<String>>::new("nickname");
        let display_alias = Field::<Option<String>>::new("displayAlias");

        assert_eq!(
            nickname.rename_to(&display_alias).into_document(),
            doc! {
                "$rename": {
                    "nickname": "displayAlias",
                },
            }
        );
    }

    #[test]
    fn null_and_exists_expressions_can_be_combined() {
        let null = Expression::comparison("nickname", ComparisonOperator::Eq, Bson::Null);

        let exists = Expression::comparison("nickname", ComparisonOperator::Exists, true);

        assert_eq!(
            (null & exists).into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": null,
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }
}
