use super::UpdateExpression;
use super::embedded_document::EmbeddedDocument;
use crate::query::expression::{ComparisonOperator, ElementExpression, Expression};
use crate::query::sort::SortExpression;
use mongodb::bson::{Bson, DateTime, Regex};
use std::{borrow::Cow, marker::PhantomData};

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
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            marker: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn from_owned(name: String) -> Self {
        Self {
            name: Cow::Owned(name),
            marker: PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn asc(&self) -> SortExpression {
        SortExpression::ascending(self.name.as_ref())
    }

    pub fn desc(&self) -> SortExpression {
        SortExpression::descending(self.name.as_ref())
    }

    pub fn exists(&self) -> Expression {
        Expression::comparison(self.name.as_ref(), ComparisonOperator::Exists, true)
    }

    pub fn not_exists(&self) -> Expression {
        Expression::comparison(self.name.as_ref(), ComparisonOperator::Exists, false)
    }

    pub fn not<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&Self) -> Expression,
    {
        Expression::not(build(self))
    }

    #[doc(hidden)]
    pub fn from_prefixed(prefix: &str, name: &str) -> Self {
        let path = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}.{name}")
        };

        Self::from_owned(path)
    }
}

impl<T> Field<Option<T>> {
    pub fn is_null(&self) -> Expression {
        Expression::comparison(self.name.as_ref(), ComparisonOperator::Eq, Bson::Null)
            & self.exists()
    }

    pub fn is_not_null(&self) -> Expression {
        Expression::comparison(self.name.as_ref(), ComparisonOperator::Ne, Bson::Null)
            & self.exists()
    }

    /// Creates a typed MongoDB `$unset` update for this optional field.
    ///
    /// The field is removed entirely from the stored MongoDB document.
    pub fn unset(&self) -> UpdateExpression {
        UpdateExpression::unset(self.name())
    }

    /// Creates a typed MongoDB `$rename` update that moves this
    /// optional field to another optional field of the same type.
    ///
    /// The source field is removed after its value is moved.
    pub fn rename_to(&self, destination: &Field<Option<T>>) -> UpdateExpression {
        UpdateExpression::rename(self.name(), destination.name())
    }
}

impl<T> Field<T>
where
    T: StringQueryValue,
{
    pub fn matches_regex(&self, pattern: impl Into<String>) -> Expression {
        Expression::comparison(
            self.name.as_ref(),
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
            self.name.as_ref(),
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

        Expression::comparison(self.name.as_ref(), operator, value)
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

        Expression::comparison(
            self.name.as_ref(),
            ComparisonOperator::In,
            Bson::Array(values),
        )
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

        Expression::comparison(
            self.name.as_ref(),
            ComparisonOperator::Nin,
            Bson::Array(values),
        )
    }

    /// Creates a typed MongoDB `$set` update for this field.
    pub fn set<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::set(self.name(), value.into())
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

        Expression::comparison(self.name.as_ref(), ComparisonOperator::Eq, value)
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

        Expression::comparison(
            self.name.as_ref(),
            ComparisonOperator::All,
            Bson::Array(values),
        )
    }

    /// Creates a typed MongoDB `$push` update that appends one value
    /// to this array field.
    pub fn push<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::push(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$addToSet` update that adds a value
    /// only when the array does not already contain it.
    pub fn add_to_set<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::add_to_set(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$pull` update that removes every
    /// occurrence of the given value from this array field.
    pub fn pull<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::pull(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$push` update using `$each` to append
    /// multiple values to this array field.
    pub fn push_each<I, V>(&self, values: I) -> UpdateExpression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        UpdateExpression::push_each(self.name(), values.into_iter().map(Into::into))
    }

    pub fn add_each_to_set<I, V>(&self, values: I) -> UpdateExpression
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
    {
        UpdateExpression::add_each_to_set(self.name(), values.into_iter().map(Into::into))
    }
}

impl<T> Field<Vec<T>> {
    pub fn has_size(&self, size: u32) -> Expression {
        Expression::comparison(
            self.name.as_ref(),
            ComparisonOperator::Size,
            Bson::Int64(i64::from(size)),
        )
    }

    pub fn elem_match<F>(&self, build: F) -> Expression
    where
        F: FnOnce(&ElementField<T>) -> ElementExpression,
    {
        let element = ElementField::new();
        let expression = build(&element);

        Expression::comparison(
            self.name.as_ref(),
            ComparisonOperator::ElemMatch,
            Bson::Document(expression.into_document()),
        )
    }

    /// Creates a typed MongoDB `$pop` update that removes the first
    /// element from this array field.
    pub fn pop_first(&self) -> UpdateExpression {
        UpdateExpression::pop(self.name(), -1)
    }

    /// Creates a typed MongoDB `$pop` update that removes the last
    /// element from this array field.
    pub fn pop_last(&self) -> UpdateExpression {
        UpdateExpression::pop(self.name(), 1)
    }
}

impl<T> Field<Vec<T>>
where
    T: EmbeddedDocument,
{
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

    /// Provides typed fields targeting the first array element matched
    /// by the query through MongoDB's positional `$` operator.
    ///
    /// The array field must be included in the query filter so MongoDB
    /// can determine which element the `$` placeholder represents.
    pub fn positional<F>(&self, build: F) -> UpdateExpression
    where
        F: FnOnce(&T::Fields) -> UpdateExpression,
    {
        let prefix = format!("{}.$", self.name());
        let fields = T::fields_with_prefix(&prefix);

        build(&fields)
    }

    // Existing elem_match_nested() and positional() methods...

    /// Provides typed fields targeting array elements identified by a
    /// MongoDB filtered positional placeholder such as `$[address]`.
    ///
    /// The corresponding array-filter condition must be supplied when
    /// executing the update.
    pub fn filtered<F>(&self, identifier: impl AsRef<str>, build: F) -> UpdateExpression
    where
        F: FnOnce(&T::Fields) -> UpdateExpression,
    {
        let prefix = format!("{}.$[{}]", self.name(), identifier.as_ref(),);

        let fields = T::fields_with_prefix(&prefix);

        build(&fields)
    }
}

impl<T> Field<T>
where
    T: EmbeddedDocument,
{
    pub fn nested<R, F>(&self, build: F) -> R
    where
        F: FnOnce(&T::Fields) -> R,
    {
        let fields = T::fields_with_prefix(self.name());

        build(&fields)
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

#[doc(hidden)]
pub trait StringQueryValue {}

impl StringQueryValue for String {}
impl StringQueryValue for Option<String> {}

#[doc(hidden)]
pub trait NumericQueryValue: Into<Bson> {}

impl NumericQueryValue for i32 {}
impl NumericQueryValue for i64 {}
impl NumericQueryValue for f64 {}

/// Marker trait for fields that support MongoDB `$currentDate` updates.
#[doc(hidden)]
pub trait DateQueryValue {}

impl DateQueryValue for DateTime {}
impl DateQueryValue for Option<DateTime> {}

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

#[doc(hidden)]
#[derive(Debug)]
pub struct ElementField<T> {
    marker: PhantomData<fn() -> T>,
}

impl<T> Copy for ElementField<T> {}

impl<T> Clone for ElementField<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ElementField<T> {
    const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> ElementField<T>
where
    T: Into<Bson>,
{
    fn comparison<V>(&self, operator: ComparisonOperator, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        let value: T = value.into();

        ElementExpression::comparison(operator, value)
    }
}

impl<T> ElementField<T>
where
    T: OrderedQueryValue + Into<Bson>,
{
    pub fn gt<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gt, value)
    }

    pub fn gte<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Gte, value)
    }

    pub fn lt<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lt, value)
    }

    pub fn lte<V>(&self, value: V) -> ElementExpression
    where
        V: Into<T>,
    {
        self.comparison(ComparisonOperator::Lte, value)
    }
}

impl<T> Field<T>
where
    T: NumericQueryValue,
{
    /// Creates a typed MongoDB `$inc` update for this numeric field.
    pub fn inc<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::inc(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$mul` update for this numeric field.
    pub fn mul<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::mul(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$min` update for this ordered field.
    ///
    /// MongoDB replaces the stored value only when the supplied value
    /// compares lower than the current value.
    pub fn min<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::min(self.name(), value.into())
    }

    /// Creates a typed MongoDB `$max` update for this ordered field.
    ///
    /// MongoDB replaces the stored value only when the supplied value
    /// compares higher than the current value.
    pub fn max<V>(&self, value: V) -> UpdateExpression
    where
        V: Into<T>,
    {
        UpdateExpression::max(self.name(), value.into())
    }
}

impl<T> Field<T>
where
    T: DateQueryValue,
{
    /// Creates a typed MongoDB `$currentDate` update that sets this
    /// field to the current BSON date and time.
    pub fn current_date(&self) -> UpdateExpression {
        UpdateExpression::current_date(self.name())
    }
}

#[cfg(test)]
mod tests {
    use crate::query::EmbeddedDocument;
    use crate::query::Expression;
    use crate::query::UpdateExpression;
    use crate::query::field::ComparisonOperator;
    use crate::query::field::RegexOption;
    use mongodb::bson::{Bson, DateTime, Regex, doc};

    use super::Field;

    struct Address;

    struct AddressFields {
        city: Field<String>,
        active: Field<bool>,
    }

    impl EmbeddedDocument for Address {
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
    fn embedded_document_array_builds_elem_match_expression() {
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
}
