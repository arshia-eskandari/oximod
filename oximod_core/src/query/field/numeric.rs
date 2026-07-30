//! Numeric, date, modulo, and bitwise field operations.

use mongodb::bson::Bson;

use super::{DateQueryValue, Field, IntegerQueryValue, NumericQueryValue};
use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::update_expression::UpdateExpression;

impl<T> Field<T>
where
    T: NumericQueryValue,
{
    /// Increments this numeric field using MongoDB `$inc`.
    ///
    /// Negative values decrement the stored value.
    pub fn inc<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::inc(self.name(), value.into())
    }

    /// Multiplies this numeric field using MongoDB `$mul`.
    pub fn mul<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::mul(self.name(), value.into())
    }

    /// Replaces the stored numeric value when `value` is lower.
    ///
    /// This produces MongoDB's `$min` update.
    pub fn min<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::min(self.name(), value.into())
    }

    /// Replaces the stored numeric value when `value` is higher.
    ///
    /// This produces MongoDB's `$max` update.
    pub fn max<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::max(self.name(), value.into())
    }

    /// Matches values whose division by `divisor` produces `remainder`.
    ///
    /// This produces MongoDB's `$mod` query operator.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.login_count.modulo(2, 0)
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn modulo<D, R>(&self, divisor: D, remainder: R) -> Expression
    where
        D: Into<T>,
        R: Into<T>,
    {
        let divisor: T = divisor.into();
        let remainder: T = remainder.into();

        Expression::comparison(
            self.name(),
            ComparisonOperator::Mod,
            Bson::Array(vec![divisor.into(), remainder.into()]),
        )
    }
}

impl<T> Field<T>
where
    T: DateQueryValue,
{
    /// Sets this field to the current BSON date and time.
    ///
    /// This produces MongoDB's `$currentDate` update.
    pub fn current_date(&self) -> UpdateExpression {
        UpdateExpression::current_date(self.name())
    }
}

impl<T> Field<T>
where
    T: IntegerQueryValue,
{
    /// Matches values where every bit set in `mask` is also set in the stored
    /// integer.
    pub fn bits_all_set<V>(&self, mask: V) -> Expression
    where
        V: Into<T>,
    {
        self.bitwise_comparison(ComparisonOperator::BitsAllSet, mask)
    }

    /// Matches values where at least one bit set in `mask` is also set in the
    /// stored integer.
    pub fn bits_any_set<V>(&self, mask: V) -> Expression
    where
        V: Into<T>,
    {
        self.bitwise_comparison(ComparisonOperator::BitsAnySet, mask)
    }

    /// Matches values where every bit set in `mask` is clear in the stored
    /// integer.
    pub fn bits_all_clear<V>(&self, mask: V) -> Expression
    where
        V: Into<T>,
    {
        self.bitwise_comparison(ComparisonOperator::BitsAllClear, mask)
    }

    /// Matches values where at least one bit set in `mask` is clear in the
    /// stored integer.
    pub fn bits_any_clear<V>(&self, mask: V) -> Expression
    where
        V: Into<T>,
    {
        self.bitwise_comparison(ComparisonOperator::BitsAnyClear, mask)
    }

    fn bitwise_comparison<V>(&self, operator: ComparisonOperator, mask: V) -> Expression
    where
        V: Into<T>,
    {
        let mask: T = mask.into();

        Expression::comparison(self.name(), operator, mask)
    }
}
