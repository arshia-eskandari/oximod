use crate::query::expression::{ComparisonOperator, Expression};
use crate::query::sort::SortExpression;
use mongodb::bson::{Bson, DateTime, Regex};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Field<T> {
    name: &'static str,
    marker: PhantomData<fn() -> T>,
}

impl<T> Copy for Field<T> {}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Field<T> {
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn asc(&self) -> SortExpression {
        SortExpression::ascending(self.name)
    }

    pub fn desc(&self) -> SortExpression {
        SortExpression::descending(self.name)
    }

    pub fn exists(&self) -> Expression {
        Expression::comparison(self.name, ComparisonOperator::Exists, true)
    }

    pub fn not_exists(&self) -> Expression {
        Expression::comparison(self.name, ComparisonOperator::Exists, false)
    }

    pub fn not<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&Self) -> Expression,
    {
        Expression::not(build(self))
    }
}

impl<T> Field<Option<T>> {
    pub fn is_null(&self) -> Expression {
        Expression::comparison(self.name, ComparisonOperator::Eq, Bson::Null) & self.exists()
    }

    pub fn is_not_null(&self) -> Expression {
        Expression::comparison(self.name, ComparisonOperator::Ne, Bson::Null) & self.exists()
    }
}

impl Field<String> {
    pub fn matches_regex(&self, pattern: impl Into<String>) -> Expression {
        Expression::comparison(
            self.name,
            ComparisonOperator::Eq,
            Bson::RegularExpression(Regex {
                pattern: pattern.into(),
                options: String::new(),
            }),
        )
    }

    pub fn matches_regex_with_options<I>(
        &self,
        pattern: impl Into<String>,
        options: I,
    ) -> Expression
    where
        I: IntoIterator<Item = RegexOption>,
    {
        let options = options
            .into_iter()
            .map(RegexOption::as_str)
            .collect::<String>();

        Expression::comparison(
            self.name,
            ComparisonOperator::Eq,
            Bson::RegularExpression(Regex {
                pattern: pattern.into(),
                options,
            }),
        )
    }

    pub fn starts_with(&self, prefix: impl AsRef<str>) -> Expression {
        let escaped_prefix = regex::escape(prefix.as_ref());

        self.matches_regex(format!("^{escaped_prefix}"))
    }

    pub fn ends_with(&self, suffix: impl AsRef<str>) -> Expression {
        let escaped_suffix = regex::escape(suffix.as_ref());

        self.matches_regex(format!("{escaped_suffix}$"))
    }

    pub fn contains_text(&self, text: impl AsRef<str>) -> Expression {
        let escaped_text = regex::escape(text.as_ref());

        self.matches_regex(escaped_text)
    }
}

impl<T> Field<T>
where
    T: Into<Bson>,
{
    pub fn eq<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Eq, value)
    }

    pub fn ne<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Ne, value)
    }

    fn comparison<V>(&self, operator: ComparisonOperator, value: V) -> Expression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        Expression::comparison(self.name, operator, value)
    }

    pub fn in_values<I, V>(&self, values: I) -> Expression
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

        Expression::comparison(self.name, ComparisonOperator::In, Bson::Array(values))
    }

    pub fn not_in_values<I, V>(&self, values: I) -> Expression
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

        Expression::comparison(self.name, ComparisonOperator::Nin, Bson::Array(values))
    }
}

impl<T> Field<Vec<T>>
where
    T: Into<Bson>,
{
    pub fn contains<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        Expression::comparison(self.name, ComparisonOperator::Eq, value)
    }

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

        Expression::comparison(self.name, ComparisonOperator::All, Bson::Array(values))
    }
}

impl<T> Field<Vec<T>> {
    pub fn has_size(&self, size: u32) -> Expression {
        Expression::comparison(
            self.name,
            ComparisonOperator::Size,
            Bson::Int64(i64::from(size)),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexOption {
    CaseInsensitive,
    Multiline,
    DotMatchesNewLine,
    IgnoreWhitespace,
}

impl RegexOption {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CaseInsensitive => "i",
            Self::Multiline => "m",
            Self::DotMatchesNewLine => "s",
            Self::IgnoreWhitespace => "x",
        }
    }
}

#[doc(hidden)]
pub trait OrderedQueryValue {}

impl OrderedQueryValue for i32 {}
impl OrderedQueryValue for i64 {}
impl OrderedQueryValue for f64 {}
impl OrderedQueryValue for String {}
impl OrderedQueryValue for DateTime {}

impl<T> Field<T>
where
    T: OrderedQueryValue + Into<Bson>,
{
    pub fn gt<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gt, value)
    }

    pub fn gte<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gte, value)
    }

    pub fn lt<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lt, value)
    }

    pub fn lte<V>(&self, value: V) -> Expression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lte, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::query::Expression;
    use crate::query::field::ComparisonOperator;
    use crate::query::field::RegexOption;
    use mongodb::bson::{Bson, Regex, doc};

    use super::Field;

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
}
