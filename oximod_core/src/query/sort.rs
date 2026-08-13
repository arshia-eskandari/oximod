//! Typed MongoDB sort expressions.
//!
//! [`SortExpression`] represents one or more ordered MongoDB sort fields.
//! Applications normally create sort expressions through methods on generated
//! [`Field`](crate::query::Field) values:
//!
//! ```ignore
//! let users = User::query()
//!     .sort_by(|user| user.role.asc())
//!     .then_sort_by(|user| user.age.desc())
//!     .all()
//!     .await?;
//! ```
//!
//! Text-search relevance sorting is created through
//! [`Query::sort_by_text_score`](crate::query::Query::sort_by_text_score).

use mongodb::bson::{Bson, Document, doc};

/// An ordered collection of MongoDB sort fields.
///
/// Sort expressions are created by:
///
/// - [`Field::asc`](crate::query::Field::asc);
/// - [`Field::desc`](crate::query::Field::desc);
/// - [`Query::sort_by_text_score`](crate::query::Query::sort_by_text_score).
///
/// Multiple expressions are combined by
/// [`Query::then_sort_by`](crate::query::Query::then_sort_by). Their insertion
/// order is preserved because MongoDB evaluates sort fields from left to
/// right.
///
/// When the same field is appended more than once, the later direction replaces
/// the earlier direction in the final MongoDB document.
///
/// # Example
///
/// ```ignore
/// let users = User::query()
///     .sort_by(|user| user.role.asc())
///     .then_sort_by(|user| user.age.desc())
///     .then_sort_by(|user| user.name.asc())
///     .all()
///     .await?;
/// ```
///
/// This creates a MongoDB sort document equivalent to:
///
/// ```text
/// {
///     "role": 1,
///     "age": -1,
///     "name": 1
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a sort expression must be attached to a query"]
pub struct SortExpression {
    fields: Vec<SortField>,
}

/// One field in an ordered sort expression.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SortField {
    name: String,
    value: SortValue,
}

/// The MongoDB value associated with a sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortValue {
    /// Sorts values in ascending order.
    Ascending,

    /// Sorts values in descending order.
    Descending,

    /// Sorts text-search results by MongoDB relevance score.
    TextScore,
}

impl SortExpression {
    /// Creates an ascending sort expression for `field`.
    pub(crate) fn ascending(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                value: SortValue::Ascending,
            }],
        }
    }

    /// Creates a descending sort expression for `field`.
    pub(crate) fn descending(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                value: SortValue::Descending,
            }],
        }
    }

    /// Creates a MongoDB text-score sort expression.
    ///
    /// The supplied field name becomes the key containing
    /// `{ "$meta": "textScore" }` in the MongoDB sort document.
    pub(crate) fn text_score(field: impl Into<String>) -> Self {
        Self {
            fields: vec![SortField {
                name: field.into(),
                value: SortValue::TextScore,
            }],
        }
    }

    /// Appends every field from `other`.
    ///
    /// Existing fields remain first, preserving MongoDB's sort precedence.
    /// A duplicate field replaces its earlier value when the final BSON
    /// document is created.
    pub(crate) fn extend(&mut self, other: Self) {
        self.fields.extend(other.fields);
    }

    /// Appends another sort expression, returning the combined expression.
    ///
    /// Existing fields keep their precedence: MongoDB evaluates sort keys
    /// from left to right, so the fields of `self` sort before the fields of
    /// `next`. When the same field appears twice, the later direction
    /// replaces the earlier one in the final BSON document.
    ///
    /// This is the value-composing form of
    /// [`Query::then_sort_by`](crate::query::Query::then_sort_by). It lets a
    /// single sort expression carry several keys, which aggregation uses to
    /// build one multi-key `$sort` stage:
    ///
    /// ```ignore
    /// let users = User::aggregate()
    ///     .sort_by(|user| user.role.asc().then(user.age.desc()))
    ///     .all()
    ///     .await?;
    /// ```
    pub fn then(mut self, next: Self) -> Self {
        self.extend(next);
        self
    }

    /// Converts this expression into a MongoDB sort document.
    ///
    /// Field insertion order is preserved. Because BSON documents cannot
    /// contain duplicate keys through this representation, a later entry for
    /// the same field replaces the earlier value.
    #[must_use]
    pub(crate) fn into_document(self) -> Document {
        let mut document = Document::new();

        for field in self.fields {
            document.insert(field.name, field.value.mongo_value());
        }

        document
    }
}

impl SortValue {
    /// Returns the BSON value expected by MongoDB for this sort type.
    #[must_use]
    fn mongo_value(self) -> Bson {
        match self {
            Self::Ascending => Bson::Int32(1),
            Self::Descending => Bson::Int32(-1),
            Self::TextScore => Bson::Document(doc! {
                "$meta": "textScore",
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::SortExpression;

    #[test]
    fn ascending_sort_converts_to_mongodb_document() {
        let sort = SortExpression::ascending("age");

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": 1,
            }
        );
    }

    #[test]
    fn descending_sort_converts_to_mongodb_document() {
        let sort = SortExpression::descending("age");

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": -1,
            }
        );
    }

    #[test]
    fn nested_field_sort_preserves_field_path() {
        let sort = SortExpression::ascending("address.city");

        assert_eq!(
            sort.into_document(),
            doc! {
                "address.city": 1,
            }
        );
    }

    #[test]
    fn sort_expressions_can_be_combined() {
        let mut sort = SortExpression::ascending("age");

        sort.extend(SortExpression::descending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": 1,
                "name": -1,
            }
        );
    }

    #[test]
    fn combined_sort_preserves_insertion_order() {
        let mut sort = SortExpression::ascending("role");

        sort.extend(SortExpression::descending("age"));
        sort.extend(SortExpression::ascending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "role": 1,
                "age": -1,
                "name": 1,
            }
        );
    }

    #[test]
    fn later_duplicate_sort_replaces_direction() {
        let mut sort = SortExpression::ascending("age");

        sort.extend(SortExpression::descending("age"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "age": -1,
            }
        );
    }

    #[test]
    fn text_score_sort_converts_to_mongodb_document() {
        let sort = SortExpression::text_score("_textScore");

        assert_eq!(
            sort.into_document(),
            doc! {
                "_textScore": {
                    "$meta": "textScore",
                },
            }
        );
    }

    #[test]
    fn text_score_sort_can_be_combined_with_field_sort() {
        let mut sort = SortExpression::text_score("_textScore");

        sort.extend(SortExpression::ascending("name"));

        assert_eq!(
            sort.into_document(),
            doc! {
                "_textScore": {
                    "$meta": "textScore",
                },
                "name": 1,
            }
        );
    }
}
