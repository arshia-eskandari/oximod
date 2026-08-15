//! Path-aware nested validation runtime used by generated models.
//!
//! `#[validate(nested)]` descends from a containing model into embedded
//! OxiMod models through the [`NestedValidate`] trait. The derive macro
//! implements the trait for `#[model(embedded)]` types, while this module
//! provides the recursive container adapters and the path helpers that keep
//! descendant error attribution unambiguous:
//!
//! - struct descent appends `.field` segments (`address.postal_code`);
//! - vector elements append ascending `[index]` segments (`items[1].sku`);
//! - map entries append quoted, escaped `["key"]` segments
//!   (`addresses["billing"].postal_code`), visited in lexicographically
//!   sorted raw-key order so error ordering stays deterministic.
//!
//! Everything here is generated-code infrastructure reached through the
//! hidden `::oximod::_feature` re-export; applications opt in with
//! `#[validate(nested)]` and never implement this trait manually.

use std::collections::HashMap;

use crate::error::oximod_error::ValidationError;

/// Collects path-aware validation errors from a nested value.
///
/// Implemented by the `Model` derive macro for `#[model(embedded)]` types,
/// and by the container adapters in this module for `Option<T>`, `Vec<T>`,
/// and `HashMap<String, T>`. A field marked `#[validate(nested)]` must
/// resolve through these containers to an embedded OxiMod model.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`#[validate(nested)]` is not supported for `{Self}`",
    label = "`#[validate(nested)]` requires an embedded OxiMod model or a supported container of one",
    note = "`nested` descends into `#[model(embedded)]` models, optionally wrapped in `Option<T>`, `Vec<T>`, or `HashMap<String, T>` (recursively combined)"
)]
pub trait NestedValidate {
    /// Appends this value's validation errors to `errors`, prefixing each
    /// error's field with `path`.
    fn collect_nested_validation_errors(&self, path: &str, errors: &mut Vec<ValidationError>);
}

impl<T: NestedValidate> NestedValidate for Option<T> {
    /// `None` produces no nested validation errors; a `required` rule on the
    /// containing field is the way to reject absence.
    fn collect_nested_validation_errors(&self, path: &str, errors: &mut Vec<ValidationError>) {
        if let Some(value) = self {
            value.collect_nested_validation_errors(path, errors);
        }
    }
}

impl<T: NestedValidate> NestedValidate for Vec<T> {
    /// Validates every element in ascending index order, attributing errors
    /// to `path[index]`.
    fn collect_nested_validation_errors(&self, path: &str, errors: &mut Vec<ValidationError>) {
        for (index, element) in self.iter().enumerate() {
            element.collect_nested_validation_errors(&format!("{path}[{index}]"), errors);
        }
    }
}

impl<T: NestedValidate> NestedValidate for HashMap<String, T> {
    /// Validates entries in lexicographically sorted raw-key order,
    /// attributing errors to `path["key"]` with the key quoted and escaped.
    ///
    /// Sorting keeps error ordering deterministic even though `HashMap`
    /// iteration order is arbitrary.
    fn collect_nested_validation_errors(&self, path: &str, errors: &mut Vec<ValidationError>) {
        let mut keys: Vec<&String> = self.keys().collect();
        keys.sort_unstable();

        for key in keys {
            self[key].collect_nested_validation_errors(
                &format!("{path}[{}]", quote_map_key(key)),
                errors,
            );
        }
    }
}

/// Joins a nested path prefix with a descendant field path.
///
/// An empty prefix returns `field` unchanged, so top-level validation keeps
/// today's flat field attribution byte-for-byte.
#[doc(hidden)]
#[inline]
pub fn join_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_owned()
    } else {
        format!("{prefix}.{field}")
    }
}

/// Quotes and escapes a map key so punctuation inside the key (dots,
/// brackets, quotes, spaces) cannot masquerade as path syntax.
fn quote_map_key(key: &str) -> String {
    let mut quoted = String::with_capacity(key.len() + 2);
    quoted.push('"');

    for ch in key.chars() {
        if matches!(ch, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(ch);
    }

    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::error::oximod_error::ValidationError;

    use super::{NestedValidate, join_path, quote_map_key};

    /// A minimal leaf standing in for a derive-generated embedded model: it
    /// reports one error attributed to its `value` field when negative.
    struct Leaf {
        value: i32,
    }

    impl NestedValidate for Leaf {
        fn collect_nested_validation_errors(&self, path: &str, errors: &mut Vec<ValidationError>) {
            if self.value < 0 {
                errors.push(ValidationError::new(
                    join_path(path, "value"),
                    "must be non-negative",
                ));
            }
        }
    }

    fn fields(errors: &[ValidationError]) -> Vec<&str> {
        errors.iter().map(|error| error.field.as_str()).collect()
    }

    #[test]
    fn join_path_keeps_top_level_fields_unprefixed() {
        assert_eq!(join_path("", "email"), "email");
        assert_eq!(join_path("address", "city"), "address.city");
        assert_eq!(join_path("items[2]", "sku"), "items[2].sku");
    }

    #[test]
    fn option_none_produces_no_errors() {
        let value: Option<Leaf> = None;
        let mut errors = Vec::new();

        value.collect_nested_validation_errors("leaf", &mut errors);

        assert!(errors.is_empty());
    }

    #[test]
    fn option_some_validates_the_inner_value() {
        let value = Some(Leaf { value: -1 });
        let mut errors = Vec::new();

        value.collect_nested_validation_errors("leaf", &mut errors);

        assert_eq!(fields(&errors), ["leaf.value"]);
    }

    #[test]
    fn vec_elements_are_indexed_in_ascending_order() {
        let value = vec![Leaf { value: -1 }, Leaf { value: 0 }, Leaf { value: -2 }];
        let mut errors = Vec::new();

        value.collect_nested_validation_errors("items", &mut errors);

        assert_eq!(fields(&errors), ["items[0].value", "items[2].value"]);
    }

    #[test]
    fn empty_vec_produces_no_errors() {
        let value: Vec<Leaf> = Vec::new();
        let mut errors = Vec::new();

        value.collect_nested_validation_errors("items", &mut errors);

        assert!(errors.is_empty());
    }

    #[test]
    fn recursive_containers_compose_paths() {
        let value = vec![Some(Leaf { value: -1 }), None];
        let mut errors = Vec::new();

        value.collect_nested_validation_errors("items", &mut errors);

        assert_eq!(fields(&errors), ["items[0].value"]);
    }

    #[test]
    fn map_entries_are_sorted_by_raw_key() {
        let mut value = HashMap::new();
        value.insert("zulu".to_owned(), Leaf { value: -1 });
        value.insert("alpha".to_owned(), Leaf { value: -1 });
        value.insert("mike".to_owned(), Leaf { value: 0 });

        let mut errors = Vec::new();

        value.collect_nested_validation_errors("entries", &mut errors);

        assert_eq!(
            fields(&errors),
            [r#"entries["alpha"].value"#, r#"entries["zulu"].value"#]
        );
    }

    #[test]
    fn map_keys_are_quoted_and_escaped() {
        assert_eq!(quote_map_key("billing"), r#""billing""#);
        assert_eq!(quote_map_key("a.b c"), r#""a.b c""#);
        assert_eq!(quote_map_key("x[0]"), r#""x[0]""#);
        assert_eq!(quote_map_key(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote_map_key(r"back\slash"), r#""back\\slash""#);
    }

    #[test]
    fn punctuated_map_keys_stay_unambiguous_in_paths() {
        let mut value = HashMap::new();
        value.insert("a.b".to_owned(), Leaf { value: -1 });

        let mut errors = Vec::new();

        value.collect_nested_validation_errors("entries", &mut errors);

        assert_eq!(fields(&errors), [r#"entries["a.b"].value"#]);
    }
}
