//! String and regular-expression field queries.
//!
//! This module provides raw MongoDB regex matching, typed regex options, and
//! escaped literal prefix, suffix, and substring helpers for required and
//! optional string fields.

use mongodb::bson::{Bson, Regex};

use super::{Field, StringQueryValue};
use crate::query::expression::{ComparisonOperator, Expression};

/// A MongoDB regular-expression option.
///
/// Options can be combined by passing an iterable to
/// [`Field::matches_regex_with_options`]. They are serialized in the order
/// supplied by the caller.
///
/// # Example
///
/// ```ignore
/// let users = User::query()
///     .filter(|user| {
///         user.name.matches_regex_with_options(
///             "^user",
///             [RegexOption::CaseInsensitive],
///         )
///     })
///     .all()
///     .await?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexOption {
    /// Performs case-insensitive matching.
    ///
    /// MongoDB option: `"i"`.
    CaseInsensitive,

    /// Causes `^` and `$` to match line boundaries.
    ///
    /// MongoDB option: `"m"`.
    Multiline,

    /// Allows `.` to match newline characters.
    ///
    /// MongoDB option: `"s"`.
    DotMatchesNewLine,

    /// Ignores unescaped whitespace and permits comments in the pattern.
    ///
    /// MongoDB option: `"x"`.
    IgnoreWhitespace,
}

impl RegexOption {
    #[must_use]
    const fn as_str(self) -> &'static str {
        match self {
            Self::CaseInsensitive => "i",
            Self::Multiline => "m",
            Self::DotMatchesNewLine => "s",
            Self::IgnoreWhitespace => "x",
        }
    }
}

impl<T> Field<T>
where
    T: StringQueryValue,
{
    /// Matches this string field using a MongoDB regular expression.
    ///
    /// The supplied pattern is passed to MongoDB unchanged.
    ///
    /// ```ignore
    /// let users = User::query()
    ///     .filter(|user| {
    ///         user.name.matches_regex("^User")
    ///     })
    ///     .all()
    ///     .await?;
    /// ```
    pub fn matches_regex(&self, pattern: impl Into<String>) -> Expression {
        self.regex_expression(pattern.into(), String::new())
    }

    /// Matches this string field using a regular expression and MongoDB regex
    /// options.
    ///
    /// The pattern is passed to MongoDB unchanged. Options are concatenated in
    /// iteration order.
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

        self.regex_expression(pattern.into(), options)
    }

    /// Matches strings beginning with `prefix`.
    ///
    /// The prefix is escaped before being inserted into the generated regular
    /// expression, so regex metacharacters are treated literally.
    pub fn starts_with(&self, prefix: impl AsRef<str>) -> Expression {
        let escaped = regex::escape(prefix.as_ref());

        self.matches_regex(format!("^{escaped}"))
    }

    /// Matches strings ending with `suffix`.
    ///
    /// The suffix is escaped before being inserted into the generated regular
    /// expression, so regex metacharacters are treated literally.
    pub fn ends_with(&self, suffix: impl AsRef<str>) -> Expression {
        let escaped = regex::escape(suffix.as_ref());

        self.matches_regex(format!("{escaped}$"))
    }

    /// Matches strings containing `text`.
    ///
    /// The supplied text is escaped, so this performs a literal substring
    /// search rather than accepting a regular-expression pattern.
    pub fn contains_text(&self, text: impl AsRef<str>) -> Expression {
        self.matches_regex(regex::escape(text.as_ref()))
    }

    fn regex_expression(&self, pattern: String, options: String) -> Expression {
        Expression::comparison(
            self.name(),
            ComparisonOperator::Eq,
            Bson::RegularExpression(Regex { pattern, options }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::RegexOption;
    use crate::query::expression::ComparisonOperator;
    use crate::query::{Expression, Field};
    use mongodb::bson::{Bson, Regex, doc};

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
}
