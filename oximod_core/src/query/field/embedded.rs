//! Embedded-model field paths and embedded-array operations.
//!
//! This module provides typed query and update methods for model fields whose
//! values implement [`FieldSchema`](crate::query::FieldSchema).
//!
//! Both collection-backed models and models declared with
//! `#[model(embedded)]` implement `FieldSchema`, allowing their generated
//! fields to participate in nested MongoDB queries and updates.

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
        let prefix = format!("{}.$[{}]", self.name(), identifier.as_ref());

        let fields = T::fields_with_prefix(&prefix);

        build(&fields)
    }

    /// Builds a typed MongoDB array-filter condition.
    ///
    /// The identifier must match the identifier passed to
    /// [`Field::filtered`].
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
