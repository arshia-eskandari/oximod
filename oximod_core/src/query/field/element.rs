//! Typed scalar array-element conditions.

use std::marker::PhantomData;

use mongodb::bson::Bson;

use super::OrderedQueryValue;
use crate::query::expression::{ComparisonOperator, ElementExpression};

/// A typed value representing one scalar array element.
///
/// This type is passed to closures used by
/// [`super::Field::elem_match`]. It has no MongoDB field name because its
/// comparisons apply directly to the array element.
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
#[doc(hidden)]
#[derive(Debug)]
pub struct ElementField<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Copy for ElementField<T> {}

impl<T> Clone for ElementField<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ElementField<T> {
    pub(crate) const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> ElementField<T>
where
    T: Into<Bson>,
{
    fn comparison<V>(&self, operator: ComparisonOperator, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        ElementExpression::comparison(operator, value)
    }
}

impl<T> ElementField<T>
where
    T: OrderedQueryValue + Into<Bson>,
{
    /// Matches array elements greater than `value`.
    pub fn gt<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gt, value)
    }

    /// Matches array elements greater than or equal to `value`.
    pub fn gte<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gte, value)
    }

    /// Matches array elements less than `value`.
    pub fn lt<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lt, value)
    }

    /// Matches array elements less than or equal to `value`.
    pub fn lte<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lte, value)
    }
}
