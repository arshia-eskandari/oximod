//! Internal typed-field value categories.
//!
//! These marker traits determine which query and update methods are available
//! for a generated [`Field`](super::Field). They are hidden implementation
//! details of OxiMod's typed-query API and are not intended to be implemented
//! or referenced by applications.

use mongodb::bson::{Bson, DateTime};

/// Marks values that support ordered comparison queries.
///
/// Implementations receive `gt`, `gte`, `lt`, and `lte` field methods.
#[doc(hidden)]
pub trait OrderedQueryValue {}

impl OrderedQueryValue for i32 {}
impl OrderedQueryValue for i64 {}
impl OrderedQueryValue for f64 {}
impl OrderedQueryValue for String {}
impl OrderedQueryValue for DateTime {}

/// Marks required and optional strings that support regular-expression
/// queries.
#[doc(hidden)]
pub trait StringQueryValue {}

impl StringQueryValue for String {}
impl StringQueryValue for Option<String> {}

/// Marks BSON-compatible numeric values that support numeric updates and
/// modulo queries.
///
/// Implementations receive `inc`, `mul`, `min`, `max`, and `modulo` field
/// methods.
#[doc(hidden)]
pub trait NumericQueryValue: Into<Bson> {}

impl NumericQueryValue for i32 {}
impl NumericQueryValue for i64 {}
impl NumericQueryValue for f64 {}

/// Marks required and optional BSON date-time values that support MongoDB
/// `$currentDate` updates.
#[doc(hidden)]
pub trait DateQueryValue {}

impl DateQueryValue for DateTime {}
impl DateQueryValue for Option<DateTime> {}

/// Marks BSON-compatible integer values that support MongoDB bitwise queries.
#[doc(hidden)]
pub trait IntegerQueryValue: Into<Bson> {}

impl IntegerQueryValue for i32 {}
impl IntegerQueryValue for i64 {}

#[cfg(test)]
mod tests {
    use super::{
        DateQueryValue, IntegerQueryValue, NumericQueryValue, OrderedQueryValue, StringQueryValue,
    };
    use mongodb::bson::DateTime;

    fn assert_ordered<T: OrderedQueryValue>() {}
    fn assert_string<T: StringQueryValue>() {}
    fn assert_numeric<T: NumericQueryValue>() {}
    fn assert_date<T: DateQueryValue>() {}
    fn assert_integer<T: IntegerQueryValue>() {}

    #[test]
    fn ordered_query_values_are_registered() {
        assert_ordered::<i32>();
        assert_ordered::<i64>();
        assert_ordered::<f64>();
        assert_ordered::<String>();
        assert_ordered::<DateTime>();
    }

    #[test]
    fn string_query_values_are_registered() {
        assert_string::<String>();
        assert_string::<Option<String>>();
    }

    #[test]
    fn numeric_query_values_are_registered() {
        assert_numeric::<i32>();
        assert_numeric::<i64>();
        assert_numeric::<f64>();
    }

    #[test]
    fn date_query_values_are_registered() {
        assert_date::<DateTime>();
        assert_date::<Option<DateTime>>();
    }

    #[test]
    fn integer_query_values_are_registered() {
        assert_integer::<i32>();
        assert_integer::<i64>();
    }
}
