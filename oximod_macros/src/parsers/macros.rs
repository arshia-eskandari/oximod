//! Shared literal-to-primitive conversion macro.
//!
//! This module supports parsers that accept several [`syn::Lit`] variants for
//! values implementing [`std::str::FromStr`]. The macro accepts either an owned
//! `Lit` or a borrowed `&Lit` and returns a `syn::Result<T>`.
//!
//! Unary negative numbers are expressions rather than literals and are handled
//! separately by `crate::parsers::literals`.

/// Parses a string, integer, or floating-point literal into the requested type.
///
/// The two-argument form generates a type-specific default diagnostic:
///
/// ```text
/// parse_lit_to_primitive_type!(literal, u32)
/// ```
///
/// The three-argument form uses the supplied diagnostic as its base message:
///
/// ```text
/// parse_lit_to_primitive_type!(
///     literal,
///     u32,
///     "expected an unsigned integer"
/// )
/// ```
///
/// Conversion failures append the error returned by the target type's
/// `FromStr` implementation. Unsupported literal kinds return only the base
/// diagnostic.
macro_rules! parse_lit_to_primitive_type {
    ($lit:expr, $ty:ty, $msg:expr) => {{
        // Taking a reference supports both `Lit` and `&Lit`; coercion removes
        // the additional reference in the borrowed case.
        let literal: &::syn::Lit = &$lit;

        match literal {
            ::syn::Lit::Str(string) => string.value().parse::<$ty>().map_err(|error| {
                ::syn::Error::new(
                    ::syn::spanned::Spanned::span(string),
                    format!("{}: {}", $msg, error),
                )
            }),
            ::syn::Lit::Int(integer) => integer.base10_digits().parse::<$ty>().map_err(|error| {
                ::syn::Error::new(
                    ::syn::spanned::Spanned::span(integer),
                    format!("{}: {}", $msg, error),
                )
            }),
            ::syn::Lit::Float(float) => float.base10_digits().parse::<$ty>().map_err(|error| {
                ::syn::Error::new(
                    ::syn::spanned::Spanned::span(float),
                    format!("{}: {}", $msg, error),
                )
            }),
            other => ::core::result::Result::Err(::syn::Error::new(
                ::syn::spanned::Spanned::span(other),
                $msg,
            )),
        }
    }};

    ($lit:expr, $ty:ty) => {{
        parse_lit_to_primitive_type!(
            $lit,
            $ty,
            concat!(
                "expected a literal compatible with type `",
                stringify!($ty),
                "`"
            )
        )
    }};
}

pub(crate) use parse_lit_to_primitive_type;

#[cfg(test)]
mod tests {
    use syn::{Lit, parse_quote};

    #[test]
    fn parses_supported_literal_kinds() {
        let string: Lit = parse_quote!("42");
        let integer: Lit = parse_quote!(17);
        let float: Lit = parse_quote!(1.25);

        let parsed_string =
            parse_lit_to_primitive_type!(string, u32).expect("string literal should parse");
        let parsed_integer =
            parse_lit_to_primitive_type!(integer, u32).expect("integer literal should parse");
        let parsed_float =
            parse_lit_to_primitive_type!(float, f64).expect("floating-point literal should parse");

        assert_eq!(parsed_string, 42);
        assert_eq!(parsed_integer, 17);
        assert_eq!(parsed_float, 1.25);
    }

    #[test]
    fn accepts_borrowed_literals() {
        let literal: Lit = parse_quote!(25);

        let value =
            parse_lit_to_primitive_type!(&literal, u32).expect("borrowed literal should parse");

        assert_eq!(value, 25);
    }

    #[test]
    fn unsupported_literals_use_the_default_message() {
        let literal: Lit = parse_quote!(true);

        let error =
            parse_lit_to_primitive_type!(literal, u32).expect_err("boolean literal should fail");

        assert_eq!(
            error.to_string(),
            "expected a literal compatible with type `u32`"
        );
    }

    #[test]
    fn unsupported_literals_preserve_a_custom_message() {
        let literal: Lit = parse_quote!(true);

        let error = parse_lit_to_primitive_type!(literal, u32, "expected an unsigned integer")
            .expect_err("boolean literal should fail");

        assert_eq!(error.to_string(), "expected an unsigned integer");
    }

    #[test]
    fn conversion_failures_include_the_base_message() {
        let literal: Lit = parse_quote!("not-a-number");

        let error = parse_lit_to_primitive_type!(literal, u32, "expected an unsigned integer")
            .expect_err("invalid numeric string should fail")
            .to_string();

        assert!(
            error.starts_with("expected an unsigned integer:"),
            "unexpected diagnostic: {error}"
        );
    }
}
