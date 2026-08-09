//! Parsed arguments for field-level MongoDB indexes.
//!
//! [`IndexArgs`] is the internal representation shared by:
//!
//! - `crate::parsers::index`, which populates it from `#[index(...)]`;
//! - `super::model_tokens`, which converts it into MongoDB index-model tokens.
//!
//! Every setting is optional so the generation layer can distinguish an
//! omitted option from one explicitly represented by the parser.

/// Internal representation of a field's `#[index(...)]` arguments.
///
/// Boolean attribute flags are normally represented as `Some(true)`. An absent
/// argument remains `None`, allowing the generated MongoDB options builder to
/// apply its normal defaults.
#[derive(Default, Debug)]
pub(crate) struct IndexArgs {
    /// Whether the index enforces uniqueness.
    pub(crate) unique: Option<bool>,

    /// Whether the index includes only documents containing the field.
    pub(crate) sparse: Option<bool>,

    /// An explicitly configured index name.
    pub(crate) name: Option<String>,

    /// The value forwarded to MongoDB's `background` index option.
    pub(crate) background: Option<bool>,

    /// The scalar index direction, normally `1` or `-1`.
    pub(crate) order: Option<i32>,

    /// The index time-to-live duration, in seconds.
    pub(crate) expire_after_secs: Option<i32>,

    /// The general MongoDB index format version.
    pub(crate) version: Option<u32>,

    /// The MongoDB text-index format version.
    pub(crate) text_index_version: Option<u32>,

    /// Whether the index is hidden from the query planner.
    pub(crate) hidden: Option<bool>,

    /// Whether the field uses a text index.
    pub(crate) text: Option<bool>,

    /// Whether the field uses a hashed index.
    pub(crate) hashed: Option<bool>,

    /// Whether the field uses a wildcard index.
    pub(crate) wildcard: Option<bool>,

    /// Whether to apply OxiMod's case-insensitive collation preset.
    pub(crate) case_insensitive: Option<bool>,

    /// The default language used by a text index.
    pub(crate) default_language: Option<String>,

    /// The document field that overrides a text index's language.
    pub(crate) language_override: Option<String>,

    /// The field's relative weight within a text index.
    pub(crate) weight: Option<u32>,

    /// Whether the field uses a spherical `2dsphere` index.
    pub(crate) geo_2dsphere: Option<bool>,

    /// Whether the field uses a planar `2d` index.
    pub(crate) geo_2d: Option<bool>,

    /// The location precision used by a `2d` index.
    pub(crate) bits: Option<u32>,

    /// The lower coordinate bound used by a `2d` index.
    pub(crate) min: Option<f64>,

    /// The upper coordinate bound used by a `2d` index.
    pub(crate) max: Option<f64>,

    /// The MongoDB `2dsphere` index format version.
    pub(crate) geo_2dsphere_index_version: Option<u32>,
}

impl IndexArgs {
    /// Returns whether these arguments declare a text index.
    ///
    /// Text-index-specific options imply a text index even when the explicit
    /// `text` flag is absent. This is the single definition shared by index
    /// key generation and cross-field declaration validation, so the two
    /// layers cannot drift.
    pub(crate) fn is_text_implying(&self) -> bool {
        self.text == Some(true)
            || self.text_index_version.is_some()
            || self.default_language.is_some()
            || self.language_override.is_some()
            || self.weight.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::IndexArgs;

    #[test]
    fn text_flag_and_every_text_associated_option_imply_a_text_index() {
        let cases = [
            IndexArgs {
                text: Some(true),
                ..IndexArgs::default()
            },
            IndexArgs {
                text_index_version: Some(2),
                ..IndexArgs::default()
            },
            IndexArgs {
                default_language: Some("english".to_string()),
                ..IndexArgs::default()
            },
            IndexArgs {
                language_override: Some("language".to_string()),
                ..IndexArgs::default()
            },
            IndexArgs {
                weight: Some(10),
                ..IndexArgs::default()
            },
        ];

        for args in cases {
            assert!(
                args.is_text_implying(),
                "expected text-implying arguments: {args:?}"
            );
        }
    }

    #[test]
    fn non_text_arguments_are_not_text_implying() {
        let cases = [
            IndexArgs::default(),
            IndexArgs {
                unique: Some(true),
                name: Some("name_idx".to_string()),
                ..IndexArgs::default()
            },
            IndexArgs {
                hashed: Some(true),
                ..IndexArgs::default()
            },
            IndexArgs {
                geo_2dsphere: Some(true),
                geo_2dsphere_index_version: Some(3),
                ..IndexArgs::default()
            },
        ];

        for args in cases {
            assert!(
                !args.is_text_implying(),
                "expected non-text arguments: {args:?}"
            );
        }
    }
}
