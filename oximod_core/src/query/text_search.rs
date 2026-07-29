use mongodb::bson::{Bson, Document};

/// Configuration for a MongoDB `$text` query.
///
/// A plain search can be created from a string:
///
/// ```
/// # use oximod::TextSearch;
/// let search = TextSearch::new("rust mongodb");
/// ```
///
/// Optional language, case-sensitivity, and diacritic-sensitivity
/// settings can be configured through the builder methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSearch {
    search: String,
    language: Option<String>,
    case_sensitive: Option<bool>,
    diacritic_sensitive: Option<bool>,
}

impl TextSearch {
    /// Creates a text-search configuration.
    pub fn new(search: impl Into<String>) -> Self {
        Self {
            search: search.into(),
            language: None,
            case_sensitive: None,
            diacritic_sensitive: None,
        }
    }

    /// Sets the language used to parse the search terms.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Controls whether the search distinguishes uppercase and
    /// lowercase characters.
    pub const fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = Some(case_sensitive);
        self
    }

    /// Controls whether the search distinguishes characters with
    /// and without diacritical marks.
    pub const fn diacritic_sensitive(mut self, diacritic_sensitive: bool) -> Self {
        self.diacritic_sensitive = Some(diacritic_sensitive);

        self
    }

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
    fn from(search: String) -> Self {
        Self::new(search)
    }
}

impl From<&str> for TextSearch {
    fn from(search: &str) -> Self {
        Self::new(search)
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::doc;

    use super::TextSearch;

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
