//! Numeric-literal parsing for validation attributes.
//!
//! Validation bounds accept integer and floating-point literals with an
//! optional leading minus sign, such as `42`, `-42`, `3.14`, or `-3.14`.
//!
//! This module also provides checked conversions used when comparing numeric
//! validation ranges during macro expansion.

use syn::{Expr, ExprLit, ExprUnary, Lit, LitFloat, LitInt, UnOp};

use crate::validate::LitNum;

/// Parses an integer or floating-point literal with an optional leading `-`.
///
/// # Errors
///
/// Returns an error when the expression is not a numeric literal or when a
/// unary minus is applied to something other than a numeric literal.
pub fn parse_num_lit_expr(expression: Expr) -> syn::Result<LitNum> {
    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Int(literal),
            ..
        }) => Ok(LitNum::Int {
            lit: literal,
            neg: false,
        }),

        Expr::Lit(ExprLit {
            lit: Lit::Float(literal),
            ..
        }) => Ok(LitNum::Float {
            lit: literal,
            neg: false,
        }),

        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => match *expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(literal),
                ..
            }) => Ok(LitNum::Int {
                lit: literal,
                neg: true,
            }),

            Expr::Lit(ExprLit {
                lit: Lit::Float(literal),
                ..
            }) => Ok(LitNum::Float {
                lit: literal,
                neg: true,
            }),

            other => Err(syn::Error::new_spanned(
                other,
                "expected a numeric literal after unary '-'",
            )),
        },

        other => Err(syn::Error::new_spanned(
            other,
            "expected a numeric literal (optionally with a leading '-')",
        )),
    }
}

/// Ensures that an integer literal is strictly greater than zero.
///
/// # Errors
///
/// Returns an error when the literal is zero or cannot be represented as a
/// `u128`.
pub fn litint_strictly_positive(literal: &LitInt) -> syn::Result<()> {
    let value = parse_u128_for_range(literal)?;

    if value == 0 {
        return Err(syn::Error::new(
            literal.span(),
            "`multiple_of` must be greater than 0",
        ));
    }

    Ok(())
}

/// Converts an integer literal into a `u128` for range comparisons.
pub fn parse_u128_for_range(literal: &LitInt) -> syn::Result<u128> {
    literal.base10_digits().parse::<u128>().map_err(|error| {
        syn::Error::new(literal.span(), format!("invalid integer literal: {error}"))
    })
}

/// Converts a floating-point literal into an `f64` for range comparisons.
pub fn parse_f64_for_range(literal: &LitFloat) -> syn::Result<f64> {
    literal
        .base10_digits()
        .parse::<f64>()
        .map_err(|error| syn::Error::new(literal.span(), format!("invalid float literal: {error}")))
}

#[cfg(test)]
mod tests {
    use syn::{Expr, LitFloat, LitInt, parse_quote};

    use crate::validate::LitNum;

    use super::{
        litint_strictly_positive, parse_f64_for_range, parse_num_lit_expr, parse_u128_for_range,
    };

    #[test]
    fn parses_positive_integer_literal() {
        let expression: Expr = parse_quote!(42);

        let parsed = parse_num_lit_expr(expression).expect("integer literal should parse");

        let LitNum::Int { lit, neg } = parsed else {
            panic!("expected an integer literal");
        };

        assert_eq!(lit.base10_digits(), "42");
        assert!(!neg);
    }

    #[test]
    fn parses_negative_integer_literal() {
        let expression: Expr = parse_quote!(-42);

        let parsed = parse_num_lit_expr(expression).expect("negative integer should parse");

        let LitNum::Int { lit, neg } = parsed else {
            panic!("expected an integer literal");
        };

        assert_eq!(lit.base10_digits(), "42");
        assert!(neg);
    }

    #[test]
    fn parses_positive_float_literal() {
        let expression: Expr = parse_quote!(3.14);

        let parsed = parse_num_lit_expr(expression).expect("float literal should parse");

        let LitNum::Float { lit, neg } = parsed else {
            panic!("expected a floating-point literal");
        };

        assert_eq!(lit.base10_digits(), "3.14");
        assert!(!neg);
    }

    #[test]
    fn parses_negative_float_literal() {
        let expression: Expr = parse_quote!(-3.14);

        let parsed = parse_num_lit_expr(expression).expect("negative float should parse");

        let LitNum::Float { lit, neg } = parsed else {
            panic!("expected a floating-point literal");
        };

        assert_eq!(lit.base10_digits(), "3.14");
        assert!(neg);
    }

    #[test]
    fn rejects_non_numeric_expressions() {
        let expression: Expr = parse_quote!(value);

        let error = parse_num_lit_expr(expression).expect_err("identifier should be rejected");

        assert_eq!(
            error.to_string(),
            "expected a numeric literal (optionally with a leading '-')"
        );
    }

    #[test]
    fn rejects_negated_non_literal_expressions() {
        let expression: Expr = parse_quote!(-value);

        let error =
            parse_num_lit_expr(expression).expect_err("negated identifier should be rejected");

        assert_eq!(
            error.to_string(),
            "expected a numeric literal after unary '-'"
        );
    }

    #[test]
    fn strictly_positive_integer_accepts_nonzero_values() {
        let literal: LitInt = parse_quote!(5);

        litint_strictly_positive(&literal).expect("positive integer should be accepted");
    }

    #[test]
    fn strictly_positive_integer_rejects_zero() {
        let literal: LitInt = parse_quote!(0);

        let error = litint_strictly_positive(&literal).expect_err("zero should be rejected");

        assert_eq!(error.to_string(), "`multiple_of` must be greater than 0");
    }

    #[test]
    fn converts_integer_literals_for_range_checks() {
        let literal: LitInt = parse_quote!(1_024);

        let value = parse_u128_for_range(&literal).expect("integer should convert");

        assert_eq!(value, 1_024);
    }

    #[test]
    fn converts_float_literals_for_range_checks() {
        let literal: LitFloat = parse_quote!(12.5);

        let value = parse_f64_for_range(&literal).expect("float should convert");

        assert_eq!(value, 12.5);
    }
}
