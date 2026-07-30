//! Scalar field queries and updates.
//!
//! This module implements equality, membership, ordered comparison, null
//! checks, `$set`, `$unset`, and `$rename`.

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
