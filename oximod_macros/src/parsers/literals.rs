use crate::validate::LitNum;
use syn::{Expr, ExprLit, ExprUnary, Lit, LitFloat, LitInt, UnOp};

/// Parse an expression that is a numeric literal, optionally with a leading `-`.
/// Accepts: `42`, `-42`, `3.14`, `-3.14`.
pub fn parse_num_lit_expr(expr: Expr) -> syn::Result<LitNum> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => Ok(LitNum::Int { lit, neg: false }),
        Expr::Lit(ExprLit {
            lit: Lit::Float(lit),
            ..
        }) => Ok(LitNum::Float { lit, neg: false }),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => {
            // Expect `-<literal>`
            match *expr {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(lit), ..
                }) => Ok(LitNum::Int { lit, neg: true }),
                Expr::Lit(ExprLit {
                    lit: Lit::Float(lit),
                    ..
                }) => Ok(LitNum::Float { lit, neg: true }),
                other => Err(syn::Error::new_spanned(
                    other,
                    "expected a numeric literal after unary '-'",
                )),
            }
        }
        other => Err(syn::Error::new_spanned(
            other,
            "expected a numeric literal (optionally with a leading '-')",
        )),
    }
}

/// Ensure an integer literal is strictly greater than zero.
/// (LitInt is always non-negative; we only need to reject zero.)
pub fn litint_strictly_positive(lit: &syn::LitInt) -> syn::Result<()> {
    // Use base10_digits() to ignore formatting/underscores/bases
    let digits = lit.base10_digits();
    // Permit non-decimal bases if you want; if not, this is fine for `base10_*` usage.
    let val: u128 = digits
        .parse()
        .map_err(|e| syn::Error::new(lit.span(), format!("invalid integer literal: {e}")))?;
    if val == 0 {
        return Err(syn::Error::new(
            lit.span(),
            "`multiple_of` must be greater than 0",
        ));
    }
    Ok(())
}

pub fn parse_u128_for_range(lit: &LitInt) -> syn::Result<u128> {
    lit.base10_digits()
        .parse::<u128>()
        .map_err(|e| syn::Error::new(lit.span(), format!("invalid integer literal: {e}")))
}

pub fn parse_f64_for_range(lit: &LitFloat) -> syn::Result<f64> {
    lit.base10_digits()
        .parse::<f64>()
        .map_err(|e| syn::Error::new(lit.span(), format!("invalid float literal: {e}")))
}
