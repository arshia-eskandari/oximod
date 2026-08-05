//! Typed MongoDB field paths.
//!
//! [`Field`] associates a serialized MongoDB field path with its Rust value
//! type. Generated field structures for collection-backed and embedded models
//! contain `Field<T>` values, exposing only the query and update operations
//! supported by `T`.
//!
//! Applications normally access fields through [`crate::query::Queryable`]
//! rather than constructing them directly.

mod array;
mod element;
mod embedded;
mod numeric;
mod scalar;
mod string;
mod traits;

use std::{borrow::Cow, marker::PhantomData};

use crate::query::bson_type::BsonType;
use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::sort::SortExpression;

pub use element::ElementField;
pub use string::RegexOption;
pub use traits::{
    DateQueryValue, IntegerQueryValue, NumericQueryValue, OrderedQueryValue, StringQueryValue,
};

/// A typed MongoDB field path.
///
/// OxiMod generates one `Field<T>` for each model field. The Rust type `T`
/// determines which query and update methods are available, while the stored
/// path respects Serde renaming and any generated nested or positional
/// prefixes.
///
/// # Example
///
/// ```ignore
/// let users = User::query()
///     .filter(|user| {
///         user.active.eq(true)
///             & user.age.gte(18)
///     })
///     .sort_by(|user| user.name.asc())
///     .all()
///     .await?;
/// ```
///
/// Embedded fields preserve their complete MongoDB path:
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
/// In that query, the generated city field targets `"address.city"`.
#[derive(Debug)]
#[must_use = "typed fields should be used to build a query, sort, or update"]
pub struct Field<T> {
    name: Cow<'static, str>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> Field<T> {
    /// Creates a field from a static MongoDB path.
    ///
    /// This constructor is used by generated model field structures.
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            marker: PhantomData,
        }
    }

    /// Creates a field from an owned MongoDB path.
    ///
    /// This constructor is used when generating nested and positional field
    /// paths dynamically.
    #[doc(hidden)]
    pub fn from_owned(name: String) -> Self {
        Self {
            name: Cow::Owned(name),
            marker: PhantomData,
        }
    }

    /// Returns the complete serialized MongoDB field path.
    ///
    /// Top-level fields return their Serde-aware field name. Nested,
    /// positional, and filtered-positional fields include their generated
    /// prefixes.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Creates an ascending sort expression for this field.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .sort_by(|user| user.name.asc())
    ///     .all()
    ///     .await?;
    /// ```
    pub fn asc(&self) -> SortExpression {
        SortExpression::ascending(self.name())
    }

    /// Creates a descending sort expression for this field.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .sort_by(|user| user.created_at.desc())
    ///     .all()
    ///     .await?;
    /// ```
    pub fn desc(&self) -> SortExpression {
        SortExpression::descending(self.name())
    }

    /// Matches documents where this field exists.
    ///
    /// This produces MongoDB's `{ "$exists": true }` condition.
    pub fn exists(&self) -> Expression {
        Expression::comparison(self.name(), ComparisonOperator::Exists, true)
    }

    /// Matches documents where this field is missing.
    ///
    /// This produces MongoDB's `{ "$exists": false }` condition.
    pub fn not_exists(&self) -> Expression {
        Expression::comparison(self.name(), ComparisonOperator::Exists, false)
    }

    /// Negates a condition built for this field.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.age.not(|age| age.gte(18))
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// Field comparisons are represented through MongoDB `$not`. Negated
    /// logical expression trees are represented through `$nor`.
    pub fn not<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&Self) -> Expression,
    {
        Expression::not(build(self))
    }

    /// Creates a field by joining a parent path and serialized field name.
    #[doc(hidden)]
    pub fn from_prefixed(prefix: &str, name: &str) -> Self {
        let path = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}.{name}")
        };

        Self::from_owned(path)
    }

    /// Matches documents where this field stores the specified BSON type.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.nickname
    ///             .has_bson_type(BsonType::String)
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    ///
    /// This checks the BSON representation stored in MongoDB. It does not
    /// deserialize or convert the field before matching.
    pub fn has_bson_type(&self, bson_type: BsonType) -> Expression {
        Expression::comparison(self.name(), ComparisonOperator::Type, bson_type)
    }
}

#[cfg(test)]
mod tests {
    use super::Field;
    use crate::query::BsonType;
    use mongodb::bson::doc;

    #[test]
    fn field_exposes_its_mongodb_name() {
        let field = Field::<String>::new("email");

        assert_eq!(field.name(), "email");
    }

    #[test]
    fn cloned_owned_field_preserves_its_mongodb_path() {
        struct Value;

        let field = Field::<Value>::from_owned("profile.address.city".to_owned());
        let cloned = field.clone();

        assert_eq!(field.name(), "profile.address.city");
        assert_eq!(cloned.name(), "profile.address.city");
    }

    #[test]
    fn nested_field_paths_are_preserved() {
        let city = Field::<String>::new("address.city");

        assert_eq!(
            city.eq("City1").into_document(),
            doc! {
                "address.city": "City1",
            }
        );
    }

    #[test]
    fn field_expressions_can_be_combined_with_and() {
        let active = Field::<bool>::new("active");
        let age = Field::<i32>::new("age");

        let expression = active.eq(true) & age.gte(18);

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "active": true,
                    },
                    {
                        "age": {
                            "$gte": 18,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn field_expressions_can_be_combined_and_nested() {
        let active = Field::<bool>::new("active");
        let age = Field::<i32>::new("age");
        let role = Field::<String>::new("role");

        let expression = active.eq(true) & (age.gte(18) | role.eq("admin"));

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "active": true,
                    },
                    {
                        "$or": [
                            {
                                "age": {
                                    "$gte": 18,
                                },
                            },
                            {
                                "role": "admin",
                            },
                        ],
                    },
                ],
            }
        );
    }

    #[test]
    fn field_builds_ascending_sort_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.asc().into_document(),
            doc! {
                "age": 1,
            }
        );
    }

    #[test]
    fn field_builds_descending_sort_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.desc().into_document(),
            doc! {
                "age": -1,
            }
        );
    }

    #[test]
    fn field_builds_exists_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.exists().into_document(),
            doc! {
                "nickname": {
                    "$exists": true,
                },
            }
        );
    }

    #[test]
    fn field_builds_not_exists_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.not_exists().into_document(),
            doc! {
                "nickname": {
                    "$exists": false,
                },
            }
        );
    }

    #[test]
    fn field_builds_not_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.not(|age| age.gte(18)).into_document(),
            doc! {
                "age": {
                    "$not": {
                        "$gte": 18,
                    },
                },
            }
        );
    }

    #[test]
    fn field_supports_an_owned_mongodb_path() {
        let city = Field::<String>::from_owned("address.city".to_owned());

        assert_eq!(city.name(), "address.city");

        assert_eq!(
            city.eq("City1").into_document(),
            doc! {
                "address.city": "City1",
            }
        );
    }

    #[test]
    fn prefixed_field_builds_nested_path() {
        let city = Field::<String>::from_prefixed("address", "city");

        assert_eq!(city.name(), "address.city");
    }

    #[test]
    fn empty_prefix_builds_relative_path() {
        let city = Field::<String>::from_prefixed("", "city");

        assert_eq!(city.name(), "city");
    }

    #[test]
    fn field_builds_bson_type_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.has_bson_type(BsonType::String).into_document(),
            doc! {
                "nickname": {
                    "$type": "string",
                },
            }
        );
    }
}
