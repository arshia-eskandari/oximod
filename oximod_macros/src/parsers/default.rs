//! Parsing of model-field default expressions.
//!
//! The `#[default(...)]` attribute accepts any valid Rust expression. The
//! expression is preserved as syntax and later inserted into the generated
//! model constructor.

use syn::{Attribute, Expr};

/// Parses the expression inside a `#[default(...)]` attribute.
///
/// # Errors
///
/// Returns a syntax error when the attribute contains no expression, contains
/// multiple comma-separated expressions, or otherwise contains invalid Rust
/// expression syntax.
pub fn parse_default_expr(attr: &Attribute) -> syn::Result<Expr> {
    attr.parse_args()
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{Attribute, parse_quote};

    use super::parse_default_expr;

    #[test]
    fn parses_literal_default_expression() {
        let attr: Attribute = parse_quote! {
            #[default("guest")]
        };

        let expression = parse_default_expr(&attr).expect("literal default should parse");

        assert_eq!(compact(&expression), "\"guest\"");
    }

    #[test]
    fn parses_function_call_default_expression() {
        let attr: Attribute = parse_quote! {
            #[default(String::from("guest"))]
        };

        let expression = parse_default_expr(&attr).expect("function-call default should parse");

        assert_eq!(compact(&expression), "String::from(\"guest\")");
    }

    #[test]
    fn parses_macro_default_expression() {
        let attr: Attribute = parse_quote! {
            #[default(vec![1, 2, 3])]
        };

        let expression = parse_default_expr(&attr).expect("macro default should parse");

        assert_eq!(compact(&expression), "vec![1,2,3]");
    }

    #[test]
    fn empty_default_attribute_is_rejected() {
        let attr: Attribute = parse_quote! {
            #[default()]
        };

        assert!(
            parse_default_expr(&attr).is_err(),
            "a default attribute must contain an expression"
        );
    }

    #[test]
    fn multiple_default_expressions_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[default("first", "second")]
        };

        assert!(
            parse_default_expr(&attr).is_err(),
            "a default attribute must contain one expression"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
