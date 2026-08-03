//! Parsing of single-value literal attributes.
//!
//! These helpers support both common attribute forms:
//!
//! - `#[attribute = "value"]`
//! - `#[attribute("value")]`
//!
//! String, integer, and floating-point literals can be converted into any
//! compatible type implementing `FromStr`.

use std::{fmt::Display, str::FromStr};

use proc_macro2::TokenStream;
use syn::{Attribute, Expr, Lit, Meta, spanned::Spanned};

use crate::parsers::macros::parse_lit_to_primitive_type;

/// Extracts the single literal contained in an attribute.
///
/// Both `#[attribute = literal]` and `#[attribute(literal)]` forms are
/// supported.
fn lit_from_attr(attr: &Attribute) -> syn::Result<Lit> {
    match &attr.meta {
        Meta::NameValue(name_value) => {
            if let Expr::Lit(expression) = &name_value.value {
                Ok(expression.lit.clone())
            } else {
                Err(syn::Error::new(attr.span(), "expected a literal after `=`"))
            }
        }

        Meta::List(_) => attr
            .parse_args::<Lit>()
            .map_err(|_| syn::Error::new(attr.span(), "expected a single literal in the list")),

        Meta::Path(_) => Err(syn::Error::new(attr.span(), "expected a literal")),
    }
}

/// Parses a single-value attribute into `T`.
///
/// When `message` is provided, it is used as the base error message for an
/// incompatible literal. Otherwise, a type-specific default message is used.
///
/// # Errors
///
/// Returns an error when the attribute does not contain exactly one supported
/// literal or when the literal cannot be parsed into `T`.
pub fn parse_attr_value<T>(attr: &Attribute, message: Option<&str>) -> syn::Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    let literal = lit_from_attr(attr)?;

    match message {
        Some(message) => {
            parse_lit_to_primitive_type!(&literal, T, message)
        }

        None => {
            parse_lit_to_primitive_type!(&literal, T)
        }
    }
}

/// Parses a single-value attribute and converts failures into compile-error
/// tokens.
pub fn parse_attr_value_ts<T>(attr: &Attribute, message: Option<&str>) -> Result<T, TokenStream>
where
    T: FromStr,
    T::Err: Display,
{
    parse_attr_value::<T>(attr, message).map_err(|error| error.to_compile_error())
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, parse_quote};

    use super::{parse_attr_value, parse_attr_value_ts};

    #[test]
    fn parses_parenthesized_string_attribute() {
        let attr: Attribute = parse_quote! {
            #[db("app_db")]
        };

        let value = parse_attr_value::<String>(&attr, None).expect("string attribute should parse");

        assert_eq!(value, "app_db");
    }

    #[test]
    fn parses_name_value_integer_attribute() {
        let attr: Attribute = parse_quote! {
            #[index_max_retries = 5]
        };

        let value = parse_attr_value::<u32>(&attr, None).expect("integer attribute should parse");

        assert_eq!(value, 5);
    }

    #[test]
    fn parses_floating_point_attribute() {
        let attr: Attribute = parse_quote! {
            #[weight(1.25)]
        };

        let value =
            parse_attr_value::<f64>(&attr, None).expect("floating-point attribute should parse");

        assert_eq!(value, 1.25);
    }

    #[test]
    fn path_attributes_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[hooks]
        };

        let error =
            parse_attr_value::<String>(&attr, None).expect_err("path attribute should fail");

        assert_eq!(error.to_string(), "expected a literal");
    }

    #[test]
    fn non_literal_name_values_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[index_max_retries = DEFAULT_RETRIES]
        };

        let error =
            parse_attr_value::<u32>(&attr, None).expect_err("non-literal value should fail");

        assert_eq!(error.to_string(), "expected a literal after `=`");
    }

    #[test]
    fn lists_must_contain_exactly_one_literal() {
        let attr: Attribute = parse_quote! {
            #[db("first", "second")]
        };

        let error =
            parse_attr_value::<String>(&attr, None).expect_err("multiple literals should fail");

        assert_eq!(error.to_string(), "expected a single literal in the list");
    }

    #[test]
    fn custom_error_messages_are_preserved() {
        let attr: Attribute = parse_quote! {
            #[index_max_retries(true)]
        };

        let error = parse_attr_value::<u32>(&attr, Some("expected retry count"))
            .expect_err("boolean should not parse as u32");

        assert_eq!(error.to_string(), "expected retry count");
    }

    #[test]
    fn token_stream_entry_point_generates_compile_error() {
        let attr: Attribute = parse_quote! {
            #[index_max_retries("invalid")]
        };

        let error = parse_attr_value_ts::<u32>(&attr, Some("expected retry count"))
            .expect_err("invalid number should fail")
            .to_string();

        assert!(
            error.contains("compile_error"),
            "error should be converted into compile-error tokens: \
             {error}"
        );

        assert!(
            error.contains("expected retry count"),
            "compile-error tokens should retain the parser message: \
             {error}"
        );
    }
}
