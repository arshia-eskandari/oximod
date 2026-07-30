//! MongoDB text-search configuration.
//!
//! [`TextSearch`] configures the top-level MongoDB `$text` query operator.
//! Applications can pass either a plain string or a configured `TextSearch`
//! value to [`Query::text`](crate::query::Query::text).
//!
//! The collection must have an appropriate text index before MongoDB can
//! execute a text-search query.

use mongodb::bson::{Bson, Document};

/// Configuration for a MongoDB `$text` query.
///
/// A basic search can be created directly from a string:
///
/// ```ignore
/// let articles = Article::query()
///     .text("rust mongodb")
///     .all()
///     .await?;
/// ```
///
/// Use `TextSearch` when additional MongoDB options are required:
///
/// ```ignore
/// use oximod::TextSearch;
///
/// let articles = Article::query()
///     .text(
///         TextSearch::new("Café")
///             .language("none")
///             .case_sensitive(true)
///             .diacritic_sensitive(true),
///     )
///     .all()
///     .await?;
/// ```
///
/// MongoDB phrase searches and excluded terms are expressed inside the search
/// string:
///
/// ```ignore
/// let articles = Article::query()
///     .text(
///         TextSearch::new(
///             "\"rust mongodb\" -beginner",
///         ),
///     )
///     .all()
///     .await?;
/// ```
///
/// Unconfigured Boolean options are omitted from the generated BSON document.
/// This distinguishes an unspecified option from one explicitly set to
/// `false`.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearch {
    search: String,
    language: Option<String>,
    case_sensitive: Option<bool>,
    diacritic_sensitive: Option<bool>,
}

impl TextSearch {
    /// Creates a text-search configuration.
    ///
    /// The search string is passed to MongoDB unchanged. It may contain
    /// ordinary terms, quoted phrases, or excluded terms using MongoDB's
    /// `$text` search syntax.
    ///
    /// # Parameters
    ///
    /// - `search`: The terms and text-search syntax to pass to MongoDB.
    ///
    /// # Example
    ///
    /// ```
    /// use oximod::TextSearch;
    ///
    /// let search = TextSearch::new(
    ///     "\"rust mongodb\" -beginner",
    /// );
    /// ```
    pub fn new(search: impl Into<String>) -> Self {
        Self {
            search: search.into(),
            language: None,
            case_sensitive: None,
            diacritic_sensitive: None,
        }
    }

    /// Sets the language used by MongoDB to parse and stem search terms.
    ///
    /// Use `"none"` to disable language-specific stemming and stop-word
    /// processing when supported by the collection's text index.
    ///
    /// ```ignore
    /// let search = TextSearch::new("running")
    ///     .language("none");
    /// ```
    ///
    /// A later call replaces the previously configured language.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Controls whether MongoDB distinguishes uppercase and lowercase
    /// characters.
    ///
    /// The option is omitted unless this method is called.
    ///
    /// ```ignore
    /// let search = TextSearch::new("Rust")
    ///     .case_sensitive(true);
    /// ```
    pub const fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = Some(case_sensitive);

        self
    }

    /// Controls whether MongoDB distinguishes characters with and without
    /// diacritical marks.
    ///
    /// The option is omitted unless this method is called.
    ///
    /// ```ignore
    /// let search = TextSearch::new("café")
    ///     .diacritic_sensitive(true);
    /// ```
    pub const fn diacritic_sensitive(mut self, diacritic_sensitive: bool) -> Self {
        self.diacritic_sensitive = Some(diacritic_sensitive);

        self
    }

    /// Converts this configuration into a top-level MongoDB `$text` document.
    pub(crate) fn into_document(self) -> Document {
        let mut options = Document::new();

        options.insert("$search", self.search);

        if let Some(language) = self.language {
            options.insert("$language", language);
        }

        if let Some(case_sensitive) = self.case_sensitive {
            options.insert("$caseSensitive", case_sensitive);
        }

        if let Some(diacritic_sensitive) = self.diacritic_sensitive {
            options.insert("$diacriticSensitive", diacritic_sensitive);
        }

        let mut document = Document::new();

        document.insert("$text", Bson::Document(options));

        document
    }
}

impl From<String> for TextSearch {
    /// Creates a basic text search from an owned string.
    fn from(search: String) -> Self {
        Self::new(search)
    }
}

impl From<&str> for TextSearch {
    /// Creates a basic text search from a borrowed string.
    fn from(search: &str) -> Self {
        Self::new(search)
    }
}

#[cfg(test)]
mod tests {
    use super::TextSearch;
    use mongodb::bson::doc;

    #[test]
    fn plain_text_search_builds_search_document() {
        assert_eq!(
            TextSearch::new("rust mongodb").into_document(),
            doc! {
                "$text": {
                    "$search": "rust mongodb",
                },
            }
        );
    }

    #[test]
    fn text_search_builds_all_options() {
        assert_eq!(
            TextSearch::new("Café Rust")
                .language("none")
                .case_sensitive(true)
                .diacritic_sensitive(true)
                .into_document(),
            doc! {
                "$text": {
                    "$search": "Café Rust",
                    "$language": "none",
                    "$caseSensitive": true,
                    "$diacriticSensitive": true,
                },
            }
        );
    }

    #[test]
    fn string_slice_converts_into_text_search() {
        let search: TextSearch = "rust".into();

        assert_eq!(
            search.into_document(),
            doc! {
                "$text": {
                    "$search": "rust",
                },
            }
        );
    }

    #[test]
    fn owned_string_converts_into_text_search() {
        let search: TextSearch = String::from("mongodb").into();

        assert_eq!(
            search.into_document(),
            doc! {
                "$text": {
                    "$search": "mongodb",
                },
            }
        );
    }

    #[test]
    fn false_options_are_preserved() {
        assert_eq!(
            TextSearch::new("rust")
                .case_sensitive(false)
                .diacritic_sensitive(false)
                .into_document(),
            doc! {
                "$text": {
                    "$search": "rust",
                    "$caseSensitive": false,
                    "$diacriticSensitive": false,
                },
            }
        );
    }
}
