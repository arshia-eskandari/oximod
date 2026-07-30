//! String and regular-expression queries.

use mongodb::bson::{Bson, Regex};

use super::{Field, StringQueryValue};
use crate::query::expression::{ComparisonOperator, Expression};

/// A MongoDB regular-expression option.
///
/// Options can be combined by passing an iterable to
/// [`Field::matches_regex_with_options`].
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

    /// Matches this string field using a regular expression and MongoDB
    /// regex options.
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
    /// expression.
    pub fn ends_with(&self, suffix: impl AsRef<str>) -> Expression {
        let escaped = regex::escape(suffix.as_ref());

        self.matches_regex(format!("{escaped}$"))
    }

    /// Matches strings containing `text`.
    ///
    /// The supplied text is escaped, so this method performs a literal
    /// substring search rather than accepting a regular-expression pattern.
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
