//! Internal value-category marker traits.
//!
//! These traits control which typed operations are available for a
//! [`Field`](super::Field). They are implementation details used by generated
//! model fields and are not intended to be implemented by applications.

use mongodb::bson::{Bson, DateTime};

/// Marks values that support ordered comparison queries.
#[doc(hidden)]
pub trait OrderedQueryValue {}

impl OrderedQueryValue for i32 {}
impl OrderedQueryValue for i64 {}
impl OrderedQueryValue for f64 {}
impl OrderedQueryValue for String {}
impl OrderedQueryValue for DateTime {}

/// Marks required and optional strings that support regex queries.
#[doc(hidden)]
pub trait StringQueryValue {}

impl StringQueryValue for String {}
impl StringQueryValue for Option<String> {}

/// Marks numeric values that support numeric queries and updates.
#[doc(hidden)]
pub trait NumericQueryValue: Into<Bson> {}

impl NumericQueryValue for i32 {}
impl NumericQueryValue for i64 {}
impl NumericQueryValue for f64 {}

/// Marks fields that support MongoDB `$currentDate` updates.
#[doc(hidden)]
pub trait DateQueryValue {}

impl DateQueryValue for DateTime {}
impl DateQueryValue for Option<DateTime> {}

/// Marks integer values that support MongoDB bitwise queries.
#[doc(hidden)]
pub trait IntegerQueryValue: Into<Bson> {}

impl IntegerQueryValue for i32 {}
impl IntegerQueryValue for i64 {}
