//! Typed model fields.
//!
//! [`Field`] represents a MongoDB field path together with its Rust value
//! type. Generated field structures for collection-backed and embedded
//! models contain `Field<T>` values, which expose only the query and update
//! operations supported by `T`.
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
/// determines which query and update methods are available.
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

    /// Returns the complete MongoDB field path.
    ///
    /// Top-level fields return their serialized field name. Nested and
    /// positional fields include their generated prefixes.
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
    use super::{Field, RegexOption};
    use crate::query::{
        BsonType, Expression, FieldSchema, UpdateExpression, expression::ComparisonOperator,
    };
    use mongodb::bson::{Bson, DateTime, Regex, doc};

    struct Address;

    struct AddressFields {
        city: Field<String>,
        active: Field<bool>,
    }

    impl FieldSchema for Address {
        type Fields = AddressFields;

        fn fields_with_prefix(prefix: &str) -> Self::Fields {
            AddressFields {
                city: Field::from_prefixed(prefix, "city"),
                active: Field::from_prefixed(prefix, "active"),
            }
        }
    }

    #[test]
    fn field_exposes_its_mongodb_name() {
        let field = Field::<String>::new("email");

        assert_eq!(field.name(), "email");
    }

    #[test]
    fn equality_builds_an_expression() {
        let active = Field::<bool>::new("active");

        assert_eq!(
            active.eq(true).into_document(),
            doc! {
                "active": true,
            }
        );
    }

    #[test]
    fn inequality_builds_an_expression() {
        let status = Field::<String>::new("status");

        assert_eq!(
            status.ne("deleted").into_document(),
            doc! {
                "status": {
                    "$ne": "deleted",
                },
            }
        );
    }

    #[test]
    fn string_field_accepts_a_string_slice() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.eq("User1").into_document(),
            doc! {
                "name": "User1",
            }
        );
    }

    #[test]
    fn greater_than_builds_an_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.gt(18).into_document(),
            doc! {
                "age": {
                    "$gt": 18,
                },
            }
        );
    }

    #[test]
    fn greater_than_or_equal_builds_an_expression() {
        let age = Field::<i32>::new("age");

        assert_eq!(
            age.gte(18).into_document(),
            doc! {
                "age": {
                    "$gte": 18,
                },
            }
        );
    }

    #[test]
    fn less_than_builds_an_expression() {
        let price = Field::<f64>::new("price");

        assert_eq!(
            price.lt(99.99).into_document(),
            doc! {
                "price": {
                    "$lt": 99.99,
                },
            }
        );
    }

    #[test]
    fn less_than_or_equal_builds_an_expression() {
        let price = Field::<i64>::new("price");

        assert_eq!(
            price.lte(100).into_document(),
            doc! {
                "price": {
                    "$lte": 100_i64,
                },
            }
        );
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
    fn field_builds_in_expression() {
        let role = Field::<String>::new("role");

        let expression = role.in_values(["admin", "moderator"]);

        assert_eq!(
            expression.into_document(),
            doc! {
                "role": {
                    "$in": [
                        "admin",
                        "moderator",
                    ],
                },
            }
        );
    }

    #[test]
    fn field_builds_not_in_expression() {
        let role = Field::<String>::new("role");

        let expression = role.not_in_values(["admin", "moderator"]);

        assert_eq!(
            expression.into_document(),
            doc! {
                "role": {
                    "$nin": [
                        "admin",
                        "moderator",
                    ],
                },
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
    fn field_builds_strict_null_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.is_null().into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": null,
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn field_builds_strict_not_null_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.is_not_null().into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": {
                            "$ne": null,
                        },
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn null_and_exists_expressions_can_be_combined() {
        let null = Expression::comparison("nickname", ComparisonOperator::Eq, Bson::Null);

        let exists = Expression::comparison("nickname", ComparisonOperator::Exists, true);

        assert_eq!(
            (null & exists).into_document(),
            doc! {
                "$and": [
                    {
                        "nickname": null,
                    },
                    {
                        "nickname": {
                            "$exists": true,
                        },
                    },
                ],
            }
        );
    }

    #[test]
    fn string_field_builds_regex_expression() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.matches_regex("^User").into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: "^User".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn regex_expression_is_inserted_directly() {
        let expression = Expression::comparison(
            "name",
            ComparisonOperator::Eq,
            Bson::RegularExpression(Regex {
                pattern: "^User".to_owned(),
                options: String::new(),
            }),
        );

        assert_eq!(
            expression.into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: "^User".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn string_field_builds_regex_with_options() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.matches_regex_with_options(
                "^user",
                [RegexOption::CaseInsensitive, RegexOption::Multiline,],
            )
            .into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: "^user".to_owned(),
                    options: "im".to_owned(),
                }),
            }
        );
    }

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
    fn string_field_builds_literal_starts_with_expression() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.starts_with("User.").into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: r"^User\.".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn string_field_builds_literal_ends_with_expression() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.ends_with(".User").into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: r"\.User$".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn string_field_builds_literal_contains_text_expression() {
        let name = Field::<String>::new("name");

        assert_eq!(
            name.contains_text("User.").into_document(),
            doc! {
                "name": Bson::RegularExpression(Regex {
                    pattern: r"User\.".to_owned(),
                    options: String::new(),
                }),
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
    fn nested_field_prefixes_embedded_field_paths() {
        let address = Field::<Address>::new("address");

        let expression =
            address.nested(|address| address.city.eq("City1") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "address.city": "City1",
                    },
                    {
                        "address.active": true,
                    },
                ],
            }
        );
    }

    #[test]
    fn optional_nested_field_prefixes_embedded_paths() {
        let address = Field::<Option<Address>>::new("address");

        let expression =
            address.nested(|address| address.city.eq("City1") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "$and": [
                    {
                        "address.city": "City1",
                    },
                    {
                        "address.active": true,
                    },
                ],
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
    fn embedded_model_array_builds_elem_match_expression() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let expression = addresses
            .elem_match_nested(|address| address.city.eq("Waterloo") & address.active.eq(true));

        assert_eq!(
            expression.into_document(),
            doc! {
                "addresses": {
                    "$elemMatch": {
                        "$and": [
                            {
                                "city": "Waterloo",
                            },
                            {
                                "active": true,
                            },
                        ],
                    },
                },
            }
        );
    }

    #[test]
    fn optional_string_field_builds_contains_text_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.contains_text("cool_").into_document(),
            doc! {
                "nickname": Bson::RegularExpression(Regex {
                    pattern: "cool_".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn optional_string_field_escapes_contains_text() {
        let nickname = Field::<Option<String>>::new("nickname");

        assert_eq!(
            nickname.contains_text("cool.").into_document(),
            doc! {
                "nickname": Bson::RegularExpression(Regex {
                    pattern: r"cool\.".to_owned(),
                    options: String::new(),
                }),
            }
        );
    }

    #[test]
    fn field_builds_set_update_expression() {
        let active = Field::<bool>::new("active");

        assert_eq!(
            active.set(true).into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
            }
        );
    }

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
    fn combines_push_update_expressions() {
        let update =
            UpdateExpression::push("tags", "mongodb") & UpdateExpression::push("scores", 100);

        assert_eq!(
            update.into_document(),
            doc! {
                "$push": {
                    "tags": "mongodb",
                    "scores": 100,
                },
            }
        );
    }

    #[test]
    fn combines_set_and_push_update_expressions() {
        let update =
            UpdateExpression::set("active", true) & UpdateExpression::push("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
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
    fn combines_add_to_set_update_expressions() {
        let update = UpdateExpression::add_to_set("tags", "mongodb")
            & UpdateExpression::add_to_set("roles", "admin");

        assert_eq!(
            update.into_document(),
            doc! {
                "$addToSet": {
                    "tags": "mongodb",
                    "roles": "admin",
                },
            }
        );
    }

    #[test]
    fn combines_set_and_add_to_set_update_expressions() {
        let update =
            UpdateExpression::set("active", true) & UpdateExpression::add_to_set("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
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
    fn combines_set_and_pull_update_expressions() {
        let update =
            UpdateExpression::set("active", false) & UpdateExpression::pull("tags", "mongodb");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": false,
                },
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
    fn combines_set_and_pop_update_expressions() {
        let update = UpdateExpression::set("active", true) & UpdateExpression::pop("tags", -1);

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
                "$pop": {
                    "tags": -1,
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
    fn optional_field_builds_rename_update_expression() {
        let nickname = Field::<Option<String>>::new("nickname");

        let display_alias = Field::<Option<String>>::new("displayAlias");

        assert_eq!(
            nickname.rename_to(&display_alias).into_document(),
            doc! {
                "$rename": {
                    "nickname": "displayAlias",
                },
            }
        );
    }

    #[test]
    fn combines_set_and_rename_update_expressions() {
        let update = UpdateExpression::set("active", true)
            & UpdateExpression::rename("nickname", "displayAlias");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
                "$rename": {
                    "nickname": "displayAlias",
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
    fn combines_set_and_current_date_update_expressions() {
        let update =
            UpdateExpression::set("active", true) & UpdateExpression::current_date("updated_at");

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "active": true,
                },
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
    fn embedded_array_positional_builds_prefixed_update_path() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.positional(|address| address.active.set(true));

        assert_eq!(
            update.into_document(),
            doc! {
            "$set": {
                "addresses.$.active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_filtered_builds_identifier_update_path() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.filtered("address", |address| address.active.set(true));

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "addresses.$[address].active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_filtered_combines_multiple_update_paths() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let update = addresses.filtered("address", |address| {
            address.city.set("City2") & address.active.set(true)
        });

        assert_eq!(
            update.into_document(),
            doc! {
                "$set": {
                    "addresses.$[address].city": "City2",
                    "addresses.$[address].active": true,
                },
            }
        );
    }

    #[test]
    fn embedded_array_builds_identifier_filter_expression() {
        let addresses = Field::<Vec<Address>>::new("addresses");

        let expression = addresses.array_filter("address", |address| address.city.eq("City1"));

        assert_eq!(
            expression.into_document(),
            doc! {
                "address.city": "City1",
            }
        );
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
