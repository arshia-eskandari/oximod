//! Array field queries and updates.

use mongodb::bson::Bson;

use super::{ElementField, Field};
use crate::query::expression::{ComparisonOperator, ElementExpression, Expression};
use crate::query::update_expression::UpdateExpression;

impl<T> Field<Vec<T>>
where
    T: Into<Bson>,
{
    /// Matches arrays containing `value`.
    ///
    /// MongoDB array membership uses ordinary equality against the array
    /// field.
    pub fn contains<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        Expression::comparison(self.name(), ComparisonOperator::Eq, value)
    }

    /// Matches arrays containing every supplied value.
    ///
    /// This produces MongoDB's `$all` operator.
    pub fn contains_all<I, V>(&self, values: I) -> Expression
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

        Expression::comparison(self.name(), ComparisonOperator::All, Bson::Array(values))
    }

    /// Appends one value to this array using MongoDB `$push`.
    pub fn push<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::push(self.name(), value.into())
    }

    /// Adds a value only when the array does not already contain it.
    ///
    /// This produces MongoDB's `$addToSet` update.
    pub fn add_to_set<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::add_to_set(self.name(), value.into())
    }

    /// Removes every occurrence of `value` from this array.
    ///
    /// This produces MongoDB's `$pull` update.
    pub fn pull<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::pull(self.name(), value.into())
    }

    /// Appends multiple values using MongoDB `$push` with `$each`.
    pub fn push_each<I, V>(&self, values: I) -> UpdateExpression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        UpdateExpression::push_each(self.name(), values.into_iter().map(Into::into))
    }

    /// Adds multiple unique values using `$addToSet` with `$each`.
    pub fn add_each_to_set<I, V>(&self, values: I) -> UpdateExpression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        UpdateExpression::add_each_to_set(self.name(), values.into_iter().map(Into::into))
    }
}

impl<T> Field<Vec<T>> {
    /// Matches arrays containing exactly `size` elements.
    ///
    /// This produces MongoDB's `$size` operator.
    pub fn has_size(&self, size: u32) -> Expression {
        Expression::comparison(
            self.name(),
            ComparisonOperator::Size,
            Bson::Int64(i64::from(size)),
        )
    }

    /// Matches arrays containing an element that satisfies every condition
    /// built by the closure.
    ///
    /// This overload is intended for scalar array elements.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.scores.elem_match(|score| {
    ///             score.gte(60)
    ///                 & score.lte(100)
    ///         })
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn elem_match<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&ElementField<T>) -> ElementExpression,
    {
        let element = ElementField::new();
        let expression = build(&element);

        Expression::comparison(
            self.name(),
            ComparisonOperator::ElemMatch,
            Bson::Document(expression.into_document()),
        )
    }

    /// Removes the first array element using MongoDB `$pop`.
    pub fn pop_first(&self) -> UpdateExpression {
        UpdateExpression::pop(self.name(), -1)
    }

    /// Removes the last array element using MongoDB `$pop`.
    pub fn pop_last(&self) -> UpdateExpression {
        UpdateExpression::pop(self.name(), 1)
    }
}
