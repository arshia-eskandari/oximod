//! Numeric, date, modulo, and bitwise field operations.
//!
//! This module provides numeric update operators, `$mod` queries, BSON
//! current-date updates, and integer bitwise query operators.

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

impl<T> Field<Option<T>>
where
    T: NumericQueryValue,
{
    /// Increments this optional numeric field using MongoDB `$inc`.
    ///
    /// The operand is a value of the inner type `T`, exactly as on a required
    /// field; `Option` operands such as `None` do not compile. The stored
    /// null or missing field follows MongoDB's normal `$inc` semantics.
    /// Negative values decrement the stored value.
    pub fn inc<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::inc(self.name(), value.into())
    }

    /// Multiplies this optional numeric field using MongoDB `$mul`.
    ///
    /// The operand is a value of the inner type `T`.
    pub fn mul<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::mul(self.name(), value.into())
    }

    /// Replaces the stored numeric value when `value` is lower.
    ///
    /// This produces MongoDB's `$min` update. The operand is a value of the
    /// inner type `T`.
    pub fn min<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::min(self.name(), value.into())
    }

    /// Replaces the stored numeric value when `value` is higher.
    ///
    /// This produces MongoDB's `$max` update. The operand is a value of the
    /// inner type `T`.
    pub fn max<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::max(self.name(), value.into())
    }

    /// Matches values whose division by `divisor` produces `remainder`.
    ///
    /// This produces MongoDB's `$mod` query operator. The operands are values
    /// of the inner type `T`.
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
    /// This produces MongoDB's `$currentDate` update using BSON type `"date"`.
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

#[cfg(test)]
mod tests {
    use crate::query::Field;
    use mongodb::bson::{DateTime, doc};

    #[test]
    fn numeric_field_builds_inc_update_expression() {
        let login_count = Field::<i32>::new("login_count");

        assert_eq!(
            login_count.inc(2).into_document(),
            doc! {
                "$inc": {
                    "login_count": 2,
                },
            }
        );
    }

    #[test]
    fn floating_point_field_builds_inc_update_expression() {
        let balance = Field::<f64>::new("balance");

        assert_eq!(
            balance.inc(1.5).into_document(),
            doc! {
                "$inc": {
                    "balance": 1.5,
                },
            }
        );
    }

    #[test]
    fn numeric_field_builds_mul_update_expression() {
        let score = Field::<i32>::new("score");

        assert_eq!(
            score.mul(3).into_document(),
            doc! {
                "$mul": {
                    "score": 3,
                },
            }
        );
    }

    #[test]
    fn floating_point_field_builds_mul_update_expression() {
        let balance = Field::<f64>::new("balance");

        assert_eq!(
            balance.mul(2.0).into_document(),
            doc! {
                "$mul": {
                    "balance": 2.0,
                },
            }
        );
    }

    #[test]
    fn ordered_field_builds_min_update_expression() {
        let score = Field::<i32>::new("score");

        assert_eq!(
            score.min(8).into_document(),
            doc! {
                "$min": {
                    "score": 8,
                },
            }
        );
    }

    #[test]
    fn floating_point_field_builds_min_update_expression() {
        let balance = Field::<f64>::new("balance");

        assert_eq!(
            balance.min(20.0).into_document(),
            doc! {
                "$min": {
                    "balance": 20.0,
                },
            }
        );
    }

    #[test]
    fn ordered_field_builds_max_update_expression() {
        let score = Field::<i32>::new("score");

        assert_eq!(
            score.max(15).into_document(),
            doc! {
                "$max": {
                    "score": 15,
                },
            }
        );
    }

    #[test]
    fn floating_point_field_builds_max_update_expression() {
        let balance = Field::<f64>::new("balance");

        assert_eq!(
            balance.max(10.0).into_document(),
            doc! {
                "$max": {
                    "balance": 10.0,
                },
            }
        );
    }

    #[test]
    fn optional_numeric_field_builds_inc_update_expression() {
        let login_count = Field::<Option<i32>>::new("login_count");

        assert_eq!(
            login_count.inc(2).into_document(),
            doc! {
                "$inc": {
                    "login_count": 2,
                },
            }
        );
    }

    #[test]
    fn optional_floating_point_field_builds_mul_update_expression() {
        let balance = Field::<Option<f64>>::new("balance");

        assert_eq!(
            balance.mul(2.0).into_document(),
            doc! {
                "$mul": {
                    "balance": 2.0,
                },
            }
        );
    }

    #[test]
    fn optional_numeric_field_builds_min_update_expression() {
        let score = Field::<Option<i32>>::new("score");

        assert_eq!(
            score.min(8).into_document(),
            doc! {
                "$min": {
                    "score": 8,
                },
            }
        );
    }

    #[test]
    fn optional_numeric_field_builds_max_update_expression() {
        let score = Field::<Option<i32>>::new("score");

        assert_eq!(
            score.max(15).into_document(),
            doc! {
                "$max": {
                    "score": 15,
                },
            }
        );
    }

    #[test]
    fn optional_numeric_field_builds_modulo_expression() {
        let login_count = Field::<Option<i32>>::new("login_count");

        assert_eq!(
            login_count.modulo(2, 0).into_document(),
            doc! {
                "login_count": {
                    "$mod": [2, 0],
                },
            }
        );
    }

    #[test]
    fn optional_int64_field_builds_modulo_expression() {
        let value = Field::<Option<i64>>::new("value");

        assert_eq!(
            value.modulo(5_i64, 1_i64).into_document(),
            doc! {
                "value": {
                    "$mod": [5_i64, 1_i64],
                },
            }
        );
    }

    #[test]
    fn date_field_builds_current_date_update_expression() {
        let updated_at = Field::<DateTime>::new("updated_at");

        assert_eq!(
            updated_at.current_date().into_document(),
            doc! {
                "$currentDate": {
                    "updated_at": {
                        "$type": "date",
                    },
                },
            }
        );
    }

    #[test]
    fn optional_date_field_builds_current_date_update_expression() {
        let updated_at = Field::<Option<DateTime>>::new("updated_at");

        assert_eq!(
            updated_at.current_date().into_document(),
            doc! {
                "$currentDate": {
                    "updated_at": {
                        "$type": "date",
                    },
                },
            }
        );
    }

    #[test]
    fn numeric_field_builds_modulo_expression() {
        let login_count = Field::<i32>::new("login_count");

        assert_eq!(
            login_count.modulo(2, 0).into_document(),
            doc! {
                "login_count": {
                    "$mod": [2, 0],
                },
            }
        );
    }

    #[test]
    fn int64_field_builds_modulo_expression() {
        let value = Field::<i64>::new("value");

        assert_eq!(
            value.modulo(5_i64, 1_i64).into_document(),
            doc! {
                "value": {
                    "$mod": [5_i64, 1_i64],
                },
            }
        );
    }

    #[test]
    fn integer_field_builds_bits_all_set_expression() {
        let permissions = Field::<i32>::new("permissions");

        assert_eq!(
            permissions.bits_all_set(0b0101).into_document(),
            doc! {
                "permissions": {
                    "$bitsAllSet": 0b0101,
                },
            }
        );
    }

    #[test]
    fn int64_field_builds_bits_all_set_expression() {
        let permissions = Field::<i64>::new("permissions");

        assert_eq!(
            permissions.bits_all_set(5_i64).into_document(),
            doc! {
                "permissions": {
                    "$bitsAllSet": 5_i64,
                },
            }
        );
    }

    #[test]
    fn integer_field_builds_bits_any_set_expression() {
        let permissions = Field::<i32>::new("permissions");

        assert_eq!(
            permissions.bits_any_set(0b1100).into_document(),
            doc! {
                "permissions": {
                    "$bitsAnySet": 0b1100,
                },
            }
        );
    }

    #[test]
    fn int64_field_builds_bits_any_set_expression() {
        let permissions = Field::<i64>::new("permissions");

        assert_eq!(
            permissions.bits_any_set(12_i64).into_document(),
            doc! {
                "permissions": {
                    "$bitsAnySet": 12_i64,
                },
            }
        );
    }

    #[test]
    fn integer_field_builds_bits_all_clear_expression() {
        let permissions = Field::<i32>::new("permissions");

        assert_eq!(
            permissions.bits_all_clear(0b1100).into_document(),
            doc! {
                "permissions": {
                    "$bitsAllClear": 0b1100,
                },
            }
        );
    }

    #[test]
    fn int64_field_builds_bits_all_clear_expression() {
        let permissions = Field::<i64>::new("permissions");

        assert_eq!(
            permissions.bits_all_clear(12_i64).into_document(),
            doc! {
                "permissions": {
                    "$bitsAllClear": 12_i64,
                },
            }
        );
    }

    #[test]
    fn integer_field_builds_bits_any_clear_expression() {
        let permissions = Field::<i32>::new("permissions");

        assert_eq!(
            permissions.bits_any_clear(0b1100).into_document(),
            doc! {
                "permissions": {
                    "$bitsAnyClear": 0b1100,
                },
            }
        );
    }

    #[test]
    fn int64_field_builds_bits_any_clear_expression() {
        let permissions = Field::<i64>::new("permissions");

        assert_eq!(
            permissions.bits_any_clear(12_i64).into_document(),
            doc! {
                "permissions": {
                    "$bitsAnyClear": 12_i64,
                },
            }
        );
    }
}
