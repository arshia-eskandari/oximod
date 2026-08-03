//! Length-validation token generation.
//!
//! Length rules support strings and collection-like fields:
//!
//! - `min_length` requires at least the configured number of elements;
//! - `max_length` permits at most the configured number of elements;
//! - `non_empty` rejects empty collections and blank strings.
//!
//! String values use `trim().is_empty()` for `non_empty`, so values
//! containing only whitespace are rejected. Other length-capable values use
//! `is_empty()`.

use quote::quote;
use syn::LitStr;

use super::super::args::{BuiltChecks, ValidateArgs};

/// Appends configured length-validation checks.
///
/// `is_string` determines whether `non_empty` should reject whitespace-only
/// values using `trim().is_empty()`.
pub fn build_length_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_name_lit: &LitStr,
    is_string: bool,
) {
    if let Some(min_length) = validate_args.min_length {
        built_checks.field_rules_val.push(quote! {
            if val.len() < (#min_length as usize) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must have a length of at least {}",
                            #min_length,
                        ),
                    ),
                );
            }
        });
    }

    if let Some(max_length) = validate_args.max_length {
        built_checks.field_rules_val.push(quote! {
            if val.len() > (#max_length as usize) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must have a length of at most {}",
                            #max_length,
                        ),
                    ),
                );
            }
        });
    }

    if validate_args.non_empty {
        let condition = if is_string {
            quote! {
                val.trim().is_empty()
            }
        } else {
            quote! {
                val.is_empty()
            }
        };

        built_checks.field_rules_val.push(quote! {
            if #condition {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be non-empty",
                    ),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::{ToTokens, quote};
    use syn::LitStr;

    use crate::validate::args::{BuiltChecks, ValidateArgs};

    use super::build_length_checks;

    #[test]
    fn absent_length_rules_generate_no_checks() {
        let mut built_checks = BuiltChecks::default();
        let validate_args = ValidateArgs::default();

        let field_name = LitStr::new("items", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, false);

        assert!(
            built_checks.field_rules_val.is_empty(),
            "fields without length rules should generate no checks"
        );
    }

    #[test]
    fn minimum_length_generates_lower_bound_check() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            min_length: Some(3),
            ..Default::default()
        };

        let field_name = LitStr::new("name", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, true);

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("ifval.len()<(3u32asusize)"),
            "minimum length should generate a lower-bound check: \
             {generated}"
        );

        assert!(
            generated.contains("\"musthavealengthofatleast{}\""),
            "minimum length should generate its error message: \
             {generated}"
        );

        assert!(
            generated.contains("\"name\""),
            "minimum length should retain the field name: \
             {generated}"
        );
    }

    #[test]
    fn maximum_length_generates_upper_bound_check() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            max_length: Some(30),
            ..Default::default()
        };

        let field_name = LitStr::new("name", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, true);

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("ifval.len()>(30u32asusize)"),
            "maximum length should generate an upper-bound check: \
             {generated}"
        );

        assert!(
            generated.contains("\"musthavealengthofatmost{}\""),
            "maximum length should generate its error message: \
             {generated}"
        );
    }

    #[test]
    fn string_non_empty_rejects_whitespace_only_values() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            non_empty: true,
            ..Default::default()
        };

        let field_name = LitStr::new("name", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, true);

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("ifval.trim().is_empty()"),
            "string non-empty checks should trim whitespace: \
             {generated}"
        );

        assert!(
            generated.contains("\"mustbenon-empty\""),
            "non-empty check should generate its error message: \
             {generated}"
        );
    }

    #[test]
    fn collection_non_empty_uses_is_empty() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            non_empty: true,
            ..Default::default()
        };

        let field_name = LitStr::new("items", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, false);

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("ifval.is_empty()"),
            "collection non-empty checks should use is_empty: \
             {generated}"
        );

        assert!(
            !generated.contains("trim()"),
            "collection checks should not use string trimming: \
             {generated}"
        );
    }

    #[test]
    fn all_length_rules_preserve_generation_order() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            min_length: Some(3),
            max_length: Some(30),
            non_empty: true,
            ..Default::default()
        };

        let field_name = LitStr::new("name", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, true);

        assert_eq!(built_checks.field_rules_val.len(), 3);

        let minimum = compact(&built_checks.field_rules_val[0]);
        let maximum = compact(&built_checks.field_rules_val[1]);
        let non_empty = compact(&built_checks.field_rules_val[2]);

        assert!(minimum.contains("val.len()<"));
        assert!(maximum.contains("val.len()>"));
        assert!(non_empty.contains("val.trim().is_empty()"));
    }

    #[test]
    fn generated_checks_append_after_existing_rules() {
        let existing = quote! {
            existing_check();
        };

        let mut built_checks = BuiltChecks {
            field_rules_val: vec![existing.clone()],
            ..Default::default()
        };

        let validate_args = ValidateArgs {
            min_length: Some(1),
            ..Default::default()
        };

        let field_name = LitStr::new("items", Span::call_site());

        build_length_checks(&mut built_checks, &validate_args, &field_name, false);

        assert_eq!(built_checks.field_rules_val.len(), 2);

        assert_eq!(
            compact(&built_checks.field_rules_val[0]),
            compact(&existing),
            "existing rules should remain unchanged"
        );

        assert!(
            compact(&built_checks.field_rules_val[1]).contains("val.len()<"),
            "new length rule should be appended"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
