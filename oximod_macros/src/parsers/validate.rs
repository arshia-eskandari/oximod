//! Parsing of field-level validation attributes.
//!
//! The `#[validate(...)]` attribute supports:
//!
//! - length rules;
//! - string-format and content rules;
//! - numeric bounds and sign rules;
//! - integer divisibility rules;
//! - required optional fields;
//! - custom validator function paths.
//!
//! This module parses attribute syntax into `ValidateArgs`. Compatibility
//! checks between validation rules and field types are performed later by the
//! validation-generation layer.

use syn::{Attribute, Expr, Lit, Path, parenthesized};

use crate::{
    parsers::literals::{litint_strictly_positive, parse_num_lit_expr},
    validate::ValidateArgs,
};

/// Parses the arguments supplied to a `#[validate(...)]` attribute.
///
/// Unrelated attributes produce an empty [`ValidateArgs`] value.
///
/// # Errors
///
/// Returns an error when:
///
/// - a validation value has the wrong literal type;
/// - a numeric bound is not a numeric literal;
/// - `multiple_of` is zero;
/// - more than one custom validator is supplied;
/// - a custom validator contains an invalid function path;
/// - an unknown validation option is supplied.
pub fn parse_validate_args(attr: &Attribute) -> syn::Result<ValidateArgs> {
    let mut args = ValidateArgs::default();

    if !attr.path().is_ident("validate") {
        return Ok(args);
    }

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("min_length") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Int(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected integer literal for `min_length`",
                ));
            };

            args.min_length = Some(literal.base10_parse::<u32>()?);
        } else if meta.path.is_ident("max_length") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Int(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected integer literal for `max_length`",
                ));
            };

            args.max_length = Some(literal.base10_parse::<u32>()?);
        } else if meta.path.is_ident("required") {
            args.required = true;
        } else if meta.path.is_ident("email") {
            args.email = true;
        } else if meta.path.is_ident("pattern") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Str(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected string literal for `pattern`",
                ));
            };

            args.pattern = Some(literal.value());
        } else if meta.path.is_ident("non_empty") {
            args.non_empty = true;
        } else if meta.path.is_ident("positive") {
            args.positive = true;
        } else if meta.path.is_ident("negative") {
            args.negative = true;
        } else if meta.path.is_ident("non_negative") {
            args.non_negative = true;
        } else if meta.path.is_ident("non_positive") {
            args.non_positive = true;
        } else if meta.path.is_ident("min") {
            let expression: Expr = meta.value()?.parse()?;

            args.min = Some(parse_num_lit_expr(expression)?);
        } else if meta.path.is_ident("max") {
            let expression: Expr = meta.value()?.parse()?;

            args.max = Some(parse_num_lit_expr(expression)?);
        } else if meta.path.is_ident("min_exclusive") {
            args.min_exclusive = true;
        } else if meta.path.is_ident("max_exclusive") {
            args.max_exclusive = true;
        } else if meta.path.is_ident("starts_with") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Str(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected string literal for `starts_with`",
                ));
            };

            args.starts_with = Some(literal.value());
        } else if meta.path.is_ident("ends_with") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Str(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected string literal for `ends_with`",
                ));
            };

            args.ends_with = Some(literal.value());
        } else if meta.path.is_ident("includes") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Str(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected string literal for `includes`",
                ));
            };

            args.includes = Some(literal.value());
        } else if meta.path.is_ident("alphanumeric") {
            args.alphanumeric = true;
        } else if meta.path.is_ident("multiple_of") {
            let literal: Lit = meta.value()?.parse()?;

            let Lit::Int(literal) = literal else {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected integer literal for `multiple_of`",
                ));
            };

            litint_strictly_positive(&literal)?;
            args.multiple_of = Some(literal);
        } else if meta.path.is_ident("custom") {
            if args.custom.is_some() {
                return Err(meta.error("duplicate `custom` validator"));
            }

            let content;
            parenthesized!(content in meta.input);

            let path: Path = content.parse()?;

            if !content.is_empty() {
                return Err(meta.error("`custom` accepts exactly one function path"));
            }

            args.custom = Some(path);
        } else {
            return Err(meta.error("unknown attribute key"));
        }

        Ok(())
    })?;

    Ok(args)
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{Attribute, parse_quote};

    use crate::validate::LitNum;

    use super::parse_validate_args;

    #[test]
    fn unrelated_attributes_produce_empty_validation_args() {
        let attr: Attribute = parse_quote! {
            #[serde(default)]
        };

        let args = parse_validate_args(&attr).expect("unrelated attribute should be ignored");

        assert!(args.min_length.is_none());
        assert!(args.max_length.is_none());
        assert!(!args.required);
        assert!(!args.email);
        assert!(args.custom.is_none());
    }

    #[test]
    fn parses_length_and_string_validations() {
        let attr: Attribute = parse_quote! {
            #[validate(
                min_length = 3,
                max_length = 30,
                non_empty,
                starts_with = "User",
                ends_with = "1",
                includes = "ser",
                alphanumeric,
                email,
                pattern = r"^[a-zA-Z0-9]+$"
            )]
        };

        let args = parse_validate_args(&attr).expect("string validations should parse");

        assert_eq!(args.min_length, Some(3));
        assert_eq!(args.max_length, Some(30));
        assert!(args.non_empty);
        assert_eq!(args.starts_with.as_deref(), Some("User"));
        assert_eq!(args.ends_with.as_deref(), Some("1"));
        assert_eq!(args.includes.as_deref(), Some("ser"));
        assert!(args.alphanumeric);
        assert!(args.email);
        assert_eq!(args.pattern.as_deref(), Some(r"^[a-zA-Z0-9]+$"));
    }

    #[test]
    fn parses_numeric_validations() {
        let attr: Attribute = parse_quote! {
            #[validate(
                min = -10,
                max = 100.5,
                min_exclusive,
                max_exclusive,
                positive,
                negative,
                non_negative,
                non_positive,
                multiple_of = 5
            )]
        };

        let args = parse_validate_args(&attr).expect("numeric validations should parse");

        let Some(LitNum::Int { lit, neg }) = args.min else {
            panic!("minimum should be an integer literal");
        };

        assert_eq!(lit.base10_digits(), "10");
        assert!(neg);

        let Some(LitNum::Float { lit, neg }) = args.max else {
            panic!("maximum should be a float literal");
        };

        assert_eq!(lit.base10_digits(), "100.5");
        assert!(!neg);

        assert!(args.min_exclusive);
        assert!(args.max_exclusive);
        assert!(args.positive);
        assert!(args.negative);
        assert!(args.non_negative);
        assert!(args.non_positive);

        assert_eq!(
            args.multiple_of
                .expect("multiple_of should be present")
                .base10_digits(),
            "5"
        );
    }

    #[test]
    fn parses_required_and_custom_validator() {
        let attr: Attribute = parse_quote! {
            #[validate(
                required,
                custom(validators::validate_name)
            )]
        };

        let args = parse_validate_args(&attr).expect("custom validator should parse");

        assert!(args.required);

        let custom = args
            .custom
            .expect("custom validator should be present")
            .to_token_stream()
            .to_string()
            .replace(' ', "");

        assert_eq!(custom, "validators::validate_name");
    }

    #[test]
    fn duplicate_custom_validators_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[validate(
                custom(validate_first),
                custom(validate_second)
            )]
        };

        let error = parse_validate_args(&attr).expect_err("duplicate custom validator should fail");

        assert_eq!(error.to_string(), "duplicate `custom` validator");
    }

    #[test]
    fn pattern_requires_a_string_literal() {
        let attr: Attribute = parse_quote! {
            #[validate(pattern = 123)]
        };

        let error = parse_validate_args(&attr).expect_err("numeric pattern should fail");

        assert_eq!(error.to_string(), "expected string literal for `pattern`");
    }

    #[test]
    fn multiple_of_must_be_strictly_positive() {
        let attr: Attribute = parse_quote! {
            #[validate(multiple_of = 0)]
        };

        let error = parse_validate_args(&attr).expect_err("zero multiple should fail");

        assert_eq!(error.to_string(), "`multiple_of` must be greater than 0");
    }

    #[test]
    fn multiple_of_requires_an_integer_literal() {
        let attr: Attribute = parse_quote! {
            #[validate(multiple_of = 2.5)]
        };

        let error = parse_validate_args(&attr).expect_err("float multiple should fail");

        assert_eq!(
            error.to_string(),
            "expected integer literal for `multiple_of`"
        );
    }

    #[test]
    fn unknown_validation_options_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[validate(unknown)]
        };

        let error = parse_validate_args(&attr).expect_err("unknown validation should fail");

        assert_eq!(error.to_string(), "unknown attribute key");
    }
}
