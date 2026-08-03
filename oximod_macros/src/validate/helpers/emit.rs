//! Numeric-literal token emission for generated validation checks.
//!
//! Parsed numeric bounds store their magnitude and sign separately. These
//! helpers reconstruct valid Rust expressions while preserving the original
//! literal where possible.
//!
//! Integer magnitudes converted for floating-point fields receive a `.0`
//! suffix so Rust infers a floating-point literal.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{LitFloat, LitInt};

/// Emits an integer literal with its optional leading minus sign.
#[inline]
pub fn emit_int_lit(literal: &LitInt, negative: bool) -> TokenStream {
    if negative {
        quote! { -#literal }
    } else {
        quote! { #literal }
    }
}

/// Emits a floating-point literal with its optional leading minus sign.
#[inline]
pub fn emit_float_from_float(literal: &LitFloat, negative: bool) -> TokenStream {
    if negative {
        quote! { -#literal }
    } else {
        quote! { #literal }
    }
}

/// Converts an integer magnitude into a floating-point expression.
///
/// The generated literal uses decimal notation with a `.0` suffix. A negative
/// value is represented as a unary-minus expression rather than placing the
/// minus sign inside the literal token.
#[inline]
pub fn emit_float_from_mag(span: Span, magnitude: u128, negative: bool) -> TokenStream {
    let representation = format!("{magnitude}.0");
    let literal = LitFloat::new(&representation, span);

    if negative {
        quote! { -#literal }
    } else {
        quote! { #literal }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::ToTokens;
    use syn::{LitFloat, LitInt, parse_quote};

    use super::{emit_float_from_float, emit_float_from_mag, emit_int_lit};

    #[test]
    fn emits_integer_literals_with_requested_sign() {
        let literal: LitInt = parse_quote!(42);

        assert_eq!(compact(&emit_int_lit(&literal, false)), "42");

        assert_eq!(compact(&emit_int_lit(&literal, true)), "-42");
    }

    #[test]
    fn emits_float_literals_with_requested_sign() {
        let literal: LitFloat = parse_quote!(12.5);

        assert_eq!(compact(&emit_float_from_float(&literal, false,)), "12.5");

        assert_eq!(compact(&emit_float_from_float(&literal, true,)), "-12.5");
    }

    #[test]
    fn converts_integer_magnitudes_into_float_literals() {
        assert_eq!(
            compact(&emit_float_from_mag(Span::call_site(), 42, false,)),
            "42.0"
        );

        assert_eq!(
            compact(&emit_float_from_mag(Span::call_site(), 42, true,)),
            "-42.0"
        );
    }

    #[test]
    fn preserves_large_integer_magnitudes() {
        let magnitude = u128::MAX;

        assert_eq!(
            compact(&emit_float_from_mag(Span::call_site(), magnitude, false,)),
            format!("{magnitude}.0")
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
