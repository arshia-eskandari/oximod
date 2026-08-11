//! Array field queries and updates.
//!
//! This module provides MongoDB array operations for [`Field<Vec<T>>`],
//! including membership checks, `$all`, `$size`, scalar `$elemMatch`, and
//! array update operators such as `$push`, `$addToSet`, `$pull`, and `$pop`.
//!
//! Operators that insert or remove values (`push`, `add_to_set`, `pull`, and
//! their `$each` variants) require the element type to implement
//! `Into<Bson>`. Scalar elements qualify automatically; derived embedded
//! models do not implement `Into<Bson>` automatically. Consumers can enable
//! these operators on a `Vec<Embedded>` field by implementing the conversion
//! once for the embedded type, for example `impl From<MyEmbedded> for Bson`
//! delegating to `bson::to_bson`. Typed *matching* on embedded arrays needs
//! no conversion — see `Field::elem_match_nested` in the embedded module.

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
    /// This overload is intended for scalar array elements. For arrays of
    /// embedded models, use [`Field::elem_match_nested`] instead.
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

#[cfg(test)]
mod tests {
    use crate::query::Field;
    use mongodb::bson::{Bson, doc};

    #[test]
    fn array_field_builds_contains_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.contains("rust").into_document(),
            doc! {
                "tags": "rust",
            }
        );
    }

    #[test]
    fn array_field_builds_contains_all_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.contains_all(["rust", "mongodb"]).into_document(),
            doc! {
                "tags": {
                    "$all": [
                        "rust",
                        "mongodb",
                    ],
                },
            }
        );
    }

    #[test]
    fn array_field_builds_size_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.has_size(2).into_document(),
            doc! {
                "tags": {
                    "$size": Bson::Int64(2),
                },
            }
        );
    }

    #[test]
    fn array_field_builds_elem_match_expression() {
        let scores = Field::<Vec<i32>>::new("scores");

        assert_eq!(
            scores
                .elem_match(|score| { score.gte(80) & score.lt(90) })
                .into_document(),
            doc! {
                "scores": {
                    "$elemMatch": {
                        "$gte": 80,
                        "$lt": 90,
                    },
                },
            }
        );
    }

    #[test]
    fn array_field_builds_push_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.push("mongodb").into_document(),
            doc! {
                "$push": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn array_field_builds_add_to_set_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.add_to_set("mongodb").into_document(),
            doc! {
                "$addToSet": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn array_field_builds_pull_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.pull("mongodb").into_document(),
            doc! {
                "$pull": {
                    "tags": "mongodb",
                },
            }
        );
    }

    #[test]
    fn array_field_builds_pop_first_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.pop_first().into_document(),
            doc! {
                "$pop": {
                    "tags": -1,
                },
            }
        );
    }

    #[test]
    fn array_field_builds_pop_last_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.pop_last().into_document(),
            doc! {
                "$pop": {
                    "tags": 1,
                },
            }
        );
    }

    #[test]
    fn array_field_builds_push_each_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.push_each(["mongodb", "backend"]).into_document(),
            doc! {
                "$push": {
                    "tags": {
                        "$each": [
                            "mongodb",
                            "backend",
                        ],
                    },
                },
            }
        );
    }

    #[test]
    fn array_field_builds_add_each_to_set_update_expression() {
        let tags = Field::<Vec<String>>::new("tags");

        assert_eq!(
            tags.add_each_to_set(["mongodb", "backend", "systems",])
                .into_document(),
            doc! {
                "$addToSet": {
                    "tags": {
                        "$each": [
                            "mongodb",
                            "backend",
                            "systems",
                        ],
                    },
                },
            }
        );
    }
}
