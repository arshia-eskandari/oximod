//! String-validation token generation.
//!
//! Supported rules include:
//!
//! - email-address format;
//! - custom regular-expression patterns;
//! - required prefixes, suffixes, and substrings;
//! - ASCII alphanumeric content.
//!
//! Regular expressions are compiled lazily in generated code through
//! `OnceLock`. User-supplied patterns are also validated during macro
//! expansion so invalid expressions produce compile-time diagnostics.

use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, LitStr};

use super::super::args::{BuiltChecks, ValidateArgs};

/// Appends configured string-validation checks.
///
/// Generated regular-expression statics include both the model and field
/// names, preventing identifier collisions between derived models.
pub fn build_string_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ident: &Ident,
    field_name_lit: &LitStr,
    struct_ident: &Ident,
) {
    if validate_args.email {
        let regex_ident = format_ident!("__oximod_email_re_{}_{}", struct_ident, field_ident,);

        let email_pattern = LitStr::new(
            r"^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?)+$",
            field_ident.span(),
        );

        built_checks.field_rules_val.push(quote! {
            #[allow(non_upper_case_globals)]
            static #regex_ident:
                ::std::sync::OnceLock<
                    ::oximod::_regex::Regex
                > = ::std::sync::OnceLock::new();

            let __re = #regex_ident.get_or_init(|| {
                ::oximod::_regex::Regex::new(
                    #email_pattern,
                )
                .unwrap()
            });

            if !__re.is_match(val) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be a valid email address",
                    ),
                );
            }
        });
    }

    if let Some(pattern) = &validate_args.pattern {
        match ::regex::Regex::new(pattern) {
            Ok(_) => {
                let pattern_lit = LitStr::new(pattern, field_ident.span());

                let regex_ident = format_ident!("__oximod_re_{}_{}", struct_ident, field_ident,);

                built_checks.field_rules_val.push(quote! {
                    #[allow(non_upper_case_globals)]
                    static #regex_ident:
                        ::std::sync::OnceLock<
                            ::oximod::_regex::Regex
                        > = ::std::sync::OnceLock::new();

                    let regex =
                        #regex_ident.get_or_init(|| {
                            ::oximod::_regex::Regex::new(
                                #pattern_lit,
                            )
                            .unwrap()
                        });

                    if !regex.is_match(val) {
                        validation_errors.push(
                            ::oximod::ValidationError::new(
                                #field_name_lit,
                                "does not match the required pattern",
                            ),
                        );
                    }
                });
            }

            Err(error) => {
                let message = format!(
                    "Invalid regex pattern in validation for \
                     '{}': {}",
                    field_ident, error,
                );

                built_checks.compile_errors.push(quote_spanned! {
                    field_ident.span() =>
                    compile_error!(#message);
                });
            }
        }
    }

    if let Some(prefix) = &validate_args.starts_with {
        built_checks.field_rules_val.push(quote! {
            if !val.starts_with(#prefix) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must start with '{}'",
                            #prefix,
                        ),
                    ),
                );
            }
        });
    }

    if let Some(suffix) = &validate_args.ends_with {
        built_checks.field_rules_val.push(quote! {
            if !val.ends_with(#suffix) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must end with '{}'",
                            #suffix,
                        ),
                    ),
                );
            }
        });
    }

    if let Some(substring) = &validate_args.includes {
        built_checks.field_rules_val.push(quote! {
            if !val.contains(#substring) {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must include '{}'",
                            #substring,
                        ),
                    ),
                );
            }
        });
    }

    if validate_args.alphanumeric {
        built_checks.field_rules_val.push(quote! {
            if !val
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric())
            {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must contain only alphanumeric characters",
                    ),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::{ToTokens, format_ident, quote};
    use syn::LitStr;

    use crate::validate::args::{BuiltChecks, ValidateArgs};

    use super::build_string_checks;

    #[test]
    fn absent_string_rules_generate_no_checks() {
        let mut built_checks = BuiltChecks::default();
        let validate_args = ValidateArgs::default();

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("name");

        let field_name = LitStr::new("name", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert!(
            built_checks.field_rules_val.is_empty(),
            "fields without string rules should generate no checks"
        );

        assert!(
            built_checks.compile_errors.is_empty(),
            "fields without string rules should generate no errors"
        );
    }

    #[test]
    fn email_rule_generates_lazily_initialized_regex() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            email: true,
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("email");

        let field_name = LitStr::new("email", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert_eq!(built_checks.field_rules_val.len(), 1);
        assert!(built_checks.compile_errors.is_empty());

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("__oximod_email_re_User_email"),
            "email regex identifier should include the model and field: \
             {generated}"
        );

        assert!(
            generated.contains("::std::sync::OnceLock<::oximod::_regex::Regex>"),
            "email regex should be initialized through OnceLock: \
             {generated}"
        );

        assert!(
            generated.contains("if!__re.is_match(val)"),
            "email rule should test the field value: {generated}"
        );

        assert!(
            generated.contains("\"mustbeavalidemailaddress\""),
            "email rule should generate its validation message: \
             {generated}"
        );
    }

    #[test]
    fn valid_pattern_generates_runtime_regex_check() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            pattern: Some(r"^[A-Z]+$".into()),
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("code");

        let field_name = LitStr::new("code", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert_eq!(built_checks.field_rules_val.len(), 1);
        assert!(built_checks.compile_errors.is_empty());

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("__oximod_re_User_code"),
            "pattern regex identifier should be unique: \
             {generated}"
        );

        assert!(
            generated.contains("\"^[A-Z]+$\""),
            "generated regex should retain the configured pattern: \
             {generated}"
        );

        assert!(
            generated.contains("\"doesnotmatchtherequiredpattern\""),
            "pattern check should generate its validation message: \
             {generated}"
        );
    }

    #[test]
    fn invalid_pattern_generates_compile_error() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            pattern: Some("(".into()),
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("code");

        let field_name = LitStr::new("code", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert!(
            built_checks.field_rules_val.is_empty(),
            "invalid patterns should not generate runtime checks"
        );

        assert_eq!(built_checks.compile_errors.len(), 1);

        let generated = compact(&built_checks.compile_errors[0]);

        assert!(
            generated.contains("compile_error!"),
            "invalid patterns should generate compile-error tokens: \
             {generated}"
        );

        assert!(
            generated.contains("Invalidregexpatterninvalidationfor'code'"),
            "diagnostic should identify the affected field: \
             {generated}"
        );
    }

    #[test]
    fn prefix_suffix_and_substring_rules_are_generated_in_order() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            starts_with: Some("User".into()),
            ends_with: Some("1".into()),
            includes: Some("ser".into()),
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("name");

        let field_name = LitStr::new("name", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert_eq!(built_checks.field_rules_val.len(), 3);

        let prefix = compact(&built_checks.field_rules_val[0]);

        let suffix = compact(&built_checks.field_rules_val[1]);

        let substring = compact(&built_checks.field_rules_val[2]);

        assert!(prefix.contains("if!val.starts_with(\"User\")"));

        assert!(suffix.contains("if!val.ends_with(\"1\")"));

        assert!(substring.contains("if!val.contains(\"ser\")"));
    }

    #[test]
    fn alphanumeric_rule_checks_ascii_characters() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            alphanumeric: true,
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("username");

        let field_name = LitStr::new("username", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("byte.is_ascii_alphanumeric()"),
            "alphanumeric validation should inspect every byte: \
             {generated}"
        );

        assert!(
            generated.contains("\"mustcontainonlyalphanumericcharacters\""),
            "alphanumeric validation should generate its message: \
             {generated}"
        );
    }

    #[test]
    fn string_checks_append_after_existing_rules() {
        let existing = quote! {
            existing_check();
        };

        let mut built_checks = BuiltChecks {
            field_rules_val: vec![existing.clone()],
            ..Default::default()
        };

        let validate_args = ValidateArgs {
            starts_with: Some("User".into()),
            ..Default::default()
        };

        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("name");

        let field_name = LitStr::new("name", Span::call_site());

        build_string_checks(
            &mut built_checks,
            &validate_args,
            &field_ident,
            &field_name,
            &struct_ident,
        );

        assert_eq!(built_checks.field_rules_val.len(), 2);

        assert_eq!(
            compact(&built_checks.field_rules_val[0]),
            compact(&existing),
            "existing rules should remain unchanged"
        );

        assert!(
            compact(&built_checks.field_rules_val[1]).contains("val.starts_with"),
            "new string rule should be appended"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
