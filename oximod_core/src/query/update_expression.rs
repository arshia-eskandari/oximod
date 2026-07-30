//! Typed MongoDB update expressions.
//!
//! [`UpdateExpression`] values are produced by methods on typed
//! [`Field`](crate::query::Field) values and passed to
//! [`Query::update_one`](crate::query::Query::update_one) or
//! [`Query::update_all`](crate::query::Query::update_all).
//!
//! Multiple updates can be combined with `&`:
//!
//! ```ignore
//! User::query()
//!     .filter(|user| user.name.eq("User1"))
//!     .update_one(|user| {
//!         user.active.set(true)
//!             & user.login_count.inc(1)
//!             & user.nickname.unset()
//!     })
//!     .await?;
//! ```
//!
//! Updates using the same MongoDB operator are merged into one operator
//! document.

use mongodb::bson::{Bson, Document, doc};
use std::ops::BitAnd;

/// A type-safe MongoDB update expression.
///
/// Update expressions are normally created through methods on generated typed
/// fields:
///
/// ```ignore
/// let updated = User::query()
///     .filter(|user| user.name.eq("User1"))
///     .update_one(|user| {
///         user.active.set(true)
///             & user.login_count.inc(1)
///     })
///     .await?;
/// ```
///
/// This produces an update document equivalent to:
///
/// ```text
/// {
///     "$set": {
///         "active": true
///     },
///     "$inc": {
///         "login_count": 1
///     }
/// }
/// ```
///
/// When expressions using the same operator and field are combined, the
/// expression on the right takes precedence:
///
/// ```ignore
/// user.score.set(10)
///     & user.score.set(20)
/// ```
///
/// The resulting `$set` value for `score` is `20`.
#[must_use]
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateExpression {
    document: Document,
}

impl UpdateExpression {
    /// Creates a MongoDB `$set` update.
    pub(crate) fn set(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$set", field, value)
    }

    /// Creates a MongoDB `$unset` update.
    ///
    /// MongoDB ignores the value associated with a field under `$unset`.
    /// OxiMod uses the conventional empty-string representation.
    pub(crate) fn unset(field: impl Into<String>) -> Self {
        single_field_update("$unset", field, "")
    }

    /// Creates a MongoDB `$inc` update.
    pub(crate) fn inc(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$inc", field, value)
    }

    /// Creates a MongoDB `$push` update.
    pub(crate) fn push(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$push", field, value)
    }

    /// Creates a MongoDB `$addToSet` update.
    pub(crate) fn add_to_set(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$addToSet", field, value)
    }

    /// Creates a MongoDB `$pull` update.
    pub(crate) fn pull(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$pull", field, value)
    }

    /// Creates a MongoDB `$pop` update.
    ///
    /// MongoDB uses `-1` to remove the first element and `1` to remove the
    /// last element.
    pub(crate) fn pop(field: impl Into<String>, position: i32) -> Self {
        single_field_update("$pop", field, position)
    }

    /// Creates a MongoDB `$push` update using `$each`.
    pub(crate) fn push_each<I>(field: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Bson>,
    {
        each_update("$push", field, values)
    }

    /// Creates a MongoDB `$addToSet` update using `$each`.
    pub(crate) fn add_each_to_set<I>(field: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Bson>,
    {
        each_update("$addToSet", field, values)
    }

    /// Creates a MongoDB `$mul` update.
    pub(crate) fn mul(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$mul", field, value)
    }

    /// Creates a MongoDB `$min` update.
    pub(crate) fn min(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$min", field, value)
    }

    /// Creates a MongoDB `$max` update.
    pub(crate) fn max(field: impl Into<String>, value: impl Into<Bson>) -> Self {
        single_field_update("$max", field, value)
    }

    /// Creates a MongoDB `$rename` update.
    pub(crate) fn rename(source: impl Into<String>, destination: impl Into<String>) -> Self {
        single_field_update("$rename", source, destination.into())
    }

    /// Creates a MongoDB `$currentDate` update using BSON date semantics.
    ///
    /// This generates:
    ///
    /// ```text
    /// {
    ///     "$currentDate": {
    ///         "updated_at": {
    ///             "$type": "date"
    ///         }
    ///     }
    /// }
    /// ```
    pub(crate) fn current_date(field: impl Into<String>) -> Self {
        single_field_update(
            "$currentDate",
            field,
            Bson::Document(doc! {
                "$type": "date",
            }),
        )
    }

    /// Converts this expression into a MongoDB update document.
    pub(crate) fn into_document(self) -> Document {
        self.document
    }
}

impl BitAnd for UpdateExpression {
    type Output = Self;

    /// Combines two update expressions.
    ///
    /// Fields using the same update operator are merged into the same operator
    /// document:
    ///
    /// ```ignore
    /// user.name.set("User1")
    ///     & user.active.set(true)
    /// ```
    ///
    /// This produces one `$set` document containing both fields.
    ///
    /// When both expressions update the same field through the same operator,
    /// the value from the expression on the right takes precedence.
    fn bitand(mut self, rhs: Self) -> Self::Output {
        for (operator, value) in rhs.document {
            match value {
                Bson::Document(rhs_fields) => {
                    if let Some(Bson::Document(lhs_fields)) = self.document.get_mut(&operator) {
                        for (field, value) in rhs_fields {
                            lhs_fields.insert(field, value);
                        }
                    } else {
                        self.document.insert(operator, Bson::Document(rhs_fields));
                    }
                }

                value => {
                    self.document.insert(operator, value);
                }
            }
        }

        self
    }
}

/// Creates an update document containing one operator and one field.
fn single_field_update(
    operator: &'static str,
    field: impl Into<String>,
    value: impl Into<Bson>,
) -> UpdateExpression {
    let mut fields = Document::new();

    fields.insert(field.into(), value.into());

    let mut document = Document::new();

    document.insert(operator, Bson::Document(fields));

    UpdateExpression { document }
}

/// Creates an array update using MongoDB's `$each` modifier.
fn each_update<I>(operator: &'static str, field: impl Into<String>, values: I) -> UpdateExpression
where
    I: IntoIterator,
    I::Item: Into<Bson>,
{
    let values = values.into_iter().map(Into::into).collect::<Vec<Bson>>();

    single_field_update(
        operator,
        field,
        Bson::Document(doc! {
            "$each": values,
        }),
    )
}

#[cfg(test)]
mod tests {
    use crate::query::Field;
    use mongodb::bson::doc;

    use super::UpdateExpression;

    #[test]
    fn set_builds_set_update_document() {
        let update = UpdateExpression::set("active", true);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
            }
        );
    }

    #[test]
    fn combines_multiple_set_update_expressions() {
        let update = UpdateExpression::set("name", "UpdatedUser")
            & UpdateExpression::set("active", true)
            & UpdateExpression::set("age", 21);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "name": "UpdatedUser",
                    "active": true,
                    "age": 21,
                },
            }
        );
    }

    #[test]
    fn rightmost_set_value_takes_precedence() {
        let update = UpdateExpression::set("age", 20) & UpdateExpression::set("age", 21);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "age": 21,
                },
            }
        );
    }

    #[test]
    fn unset_builds_unset_update_document() {
        let update = UpdateExpression::unset("nickname");

        assert_eq!(
            update.into_document(),
            doc! {
                "$unset": {
                    "nickname": "",
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
    fn combines_set_and_unset_update_expressions() {
        let update = UpdateExpression::set("active", false) & UpdateExpression::unset("nickname");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": false,
                },
                "$unset": {
                    "nickname": "",
                },
            }
        );
    }

    #[test]
    fn inc_builds_inc_update_document() {
        let update = UpdateExpression::inc("login_count", 2);

        assert_eq!(
            update.into_document(),
            doc! {
                "$inc": {
                    "login_count": 2,
                },
            }
        );
    }

    #[test]
    fn combines_set_unset_and_inc_update_expressions() {
        let update = UpdateExpression::set("active", true)
            & UpdateExpression::unset("nickname")
            & UpdateExpression::inc("login_count", 1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
                "$unset": {
                    "nickname": "",
                },
                "$inc": {
                    "login_count": 1,
                },
            }
        );
    }

    #[test]
    fn push_builds_push_update_document() {
        let update = UpdateExpression::push("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$push": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn add_to_set_builds_add_to_set_update_document() {
        let update = UpdateExpression::add_to_set("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$addToSet": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn pull_builds_pull_update_document() {
        let update = UpdateExpression::pull("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$pull": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn pop_first_builds_pop_update_document() {
        let update = UpdateExpression::pop("tags", -1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$pop": {
                    "tags": -1,
                },
            }
        );
    }

    #[test]
    fn pop_last_builds_pop_update_document() {
        let update = UpdateExpression::pop("tags", 1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$pop": {
                    "tags": 1,
                },
            }
        );
    }

    #[test]
    fn mul_builds_mul_update_document() {
        let update = UpdateExpression::mul("score", 3);

        assert_eq!(
            update.into_document(),
            doc! {
                "$mul": {
                    "score": 3,
                },
            }
        );
    }

    #[test]
    fn combines_multiple_mul_update_expressions() {
        let update = UpdateExpression::mul("score", 3) & UpdateExpression::mul("balance", 2.0);

        assert_eq!(
            update.into_document(),
            doc! {
                "$mul": {
                    "score": 3,
                    "balance": 2.0,
                },
            }
        );
    }

    #[test]
    fn min_builds_min_update_document() {
        let update = UpdateExpression::min("score", 8);

        assert_eq!(
            update.into_document(),
            doc! {
                "$min": {
                    "score": 8,
                },
            }
        );
    }

    #[test]
    fn combines_multiple_min_update_expressions() {
        let update = UpdateExpression::min("score", 8) & UpdateExpression::min("balance", 20.0);

        assert_eq!(
            update.into_document(),
            doc! {
                "$min": {
                    "score": 8,
                    "balance": 20.0,
                },
            }
        );
    }

    #[test]
    fn max_builds_max_update_document() {
        let update = UpdateExpression::max("score", 15);

        assert_eq!(
            update.into_document(),
            doc! {
                "$max": {
                    "score": 15,
                },
            }
        );
    }

    #[test]
    fn combines_multiple_max_update_expressions() {
        let update = UpdateExpression::max("score", 15) & UpdateExpression::max("balance", 10.0);

        assert_eq!(
            update.into_document(),
            doc! {
                "$max": {
                    "score": 15,
                    "balance": 10.0,
                },
            }
        );
    }

    #[test]
    fn rename_builds_rename_update_document() {
        let update = UpdateExpression::rename("nickname", "displayAlias");

        assert_eq!(
            update.into_document(),
            doc! {
                "$rename": {
                    "nickname": "displayAlias",
                },
            }
        );
    }

    #[test]
    fn current_date_builds_current_date_update_document() {
        let update = UpdateExpression::current_date("updated_at");

        assert_eq!(
            update.into_document(),
            doc! {
                "$currentDate": {
                    "updated_at": {
                        "$type": "date",
                    },
                },
            }
        );
    }
}
