//! Numeric-bound validation and token generation.
//!
//! This module:
//!
//! - checks whether validation bounds fit their field's primitive type;
//! - emits correctly typed numeric expressions;
//! - generates `multiple_of` operands and unsigned power-of-two masks;
//! - compares parsed numeric literals during macro expansion.
//!
//! Integer comparisons use sign-and-magnitude ordering so the complete
//! `u128` range can be compared without narrowing values to `i128`.

use crate::parsers::{parse_f64_for_range, parse_u128_for_range};
use crate::validate::args::{LitNum, PrimitiveNum};
use crate::validate::helpers::{
    emit_float_from_float, emit_float_from_mag, emit_int_lit, is_signed,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::cmp::Ordering;
use syn::{Ident, LitInt, Result};

fn check_int_fits_primitive(
    span: Span,
    neg: bool,
    mag: u128,
    prim: PrimitiveNum,
) -> Option<TokenStream> {
    use PrimitiveNum::*;
    let err = |msg: &str| Some(quote_spanned! { span => compile_error!(#msg); });

    let fits_signed = |min: i128, max: i128| -> bool {
        let m_i128 = match i128::try_from(mag) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let val = if neg { -m_i128 } else { m_i128 };
        val >= min && val <= max
    };

    let fits_unsigned = |max: u128| -> bool {
        if neg {
            return false;
        }
        mag <= max
    };

    match prim {
        I8 => {
            if !fits_signed(i8::MIN as i128, i8::MAX as i128) {
                return err("numeric bound does not fit `i8`");
            }
        }
        I16 => {
            if !fits_signed(i16::MIN as i128, i16::MAX as i128) {
                return err("numeric bound does not fit `i16`");
            }
        }
        I32 => {
            if !fits_signed(i32::MIN as i128, i32::MAX as i128) {
                return err("numeric bound does not fit `i32`");
            }
        }
        I64 => {
            if !fits_signed(i64::MIN as i128, i64::MAX as i128) {
                return err("numeric bound does not fit `i64`");
            }
        }
        I128 => {
            if neg {
                if mag > (i128::MAX as u128) + 1 {
                    return err("numeric bound does not fit `i128`");
                }
            } else if mag > i128::MAX as u128 {
                return err("numeric bound does not fit `i128`");
            }
        }
        Isize => { /* let rustc enforce target width */ }

        U8 => {
            if !fits_unsigned(u8::MAX as u128) {
                return err("numeric bound does not fit `u8`");
            }
        }
        U16 => {
            if !fits_unsigned(u16::MAX as u128) {
                return err("numeric bound does not fit `u16`");
            }
        }
        U32 => {
            if !fits_unsigned(u32::MAX as u128) {
                return err("numeric bound does not fit `u32`");
            }
        }
        U64 => {
            if !fits_unsigned(u64::MAX as u128) {
                return err("numeric bound does not fit `u64`");
            }
        }
        U128 => {
            if neg {
                return err("negative bound not allowed for unsigned type");
            }
        }
        Usize => {
            if neg {
                return err("negative bound not allowed for unsigned type");
            }
        }

        F32 | F64 | PrimitiveNum::NonNumeric => {}
    }
    None
}

fn check_float_fits_primitive(span: Span, v: f64, prim: PrimitiveNum) -> Option<TokenStream> {
    use PrimitiveNum::*;
    let err = |msg: &str| Some(quote_spanned! { span => compile_error!(#msg); });

    if !v.is_finite() {
        return err("float bound must be finite");
    }
    match prim {
        F32 => {
            if v < f32::MIN as f64 || v > f32::MAX as f64 {
                return err("float bound does not fit `f32`");
            }
        }
        F64 => { /* any finite f64 is OK */ }
        _ => {}
    }
    None
}

/// Produce a RHS numeric token appropriate for the field's `prim` type from a `LitNum` bound.
/// - Emits compile_error! into `compile_errors` if the field is non-numeric,
///   or the literal doesn't fit the field's primitive.
/// - Returns Some(rhs_tokens) if OK, None if a compile_error! was emitted.
///
/// Requirements:
/// - `emit_int_lit`, `emit_float_from_int`, `emit_float_from_float`
/// - `parse_u128_for_range`, `parse_f64_for_range`
/// - `check_int_fits_primitive`, `check_float_fits_primitive`
/// - `PrimitiveNum` enum + `primitive_of(...)` already in your module
pub fn rhs_for_numeric_bound(
    prim: PrimitiveNum,
    bound: &LitNum,
    field_ident: &Ident,
    compile_errors: &mut Vec<TokenStream>,
) -> Option<TokenStream> {
    match (prim, bound) {
        (PrimitiveNum::NonNumeric, _) => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(min)]`/`max` can only be applied to numeric fields");
            });
            None
        }

        (
            PrimitiveNum::I8
            | PrimitiveNum::I16
            | PrimitiveNum::I32
            | PrimitiveNum::I64
            | PrimitiveNum::I128
            | PrimitiveNum::Isize
            | PrimitiveNum::U8
            | PrimitiveNum::U16
            | PrimitiveNum::U32
            | PrimitiveNum::U64
            | PrimitiveNum::U128
            | PrimitiveNum::Usize,
            &LitNum::Int { ref lit, neg },
        ) => {
            match parse_u128_for_range(lit) {
                Ok(mag) => {
                    if let Some(err) = check_int_fits_primitive(lit.span(), neg, mag, prim) {
                        compile_errors.push(err);
                        return None;
                    }
                }
                Err(e) => {
                    compile_errors.push(e.to_compile_error());
                    return None;
                }
            }
            Some(emit_int_lit(lit, neg))
        }

        (
            PrimitiveNum::I8
            | PrimitiveNum::I16
            | PrimitiveNum::I32
            | PrimitiveNum::I64
            | PrimitiveNum::I128
            | PrimitiveNum::Isize
            | PrimitiveNum::U8
            | PrimitiveNum::U16
            | PrimitiveNum::U32
            | PrimitiveNum::U64
            | PrimitiveNum::U128
            | PrimitiveNum::Usize,
            &LitNum::Float { .. },
        ) => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("float literal is not allowed for integer field in `#[validate(min)]`/`max`");
            });
            None
        }

        (PrimitiveNum::F32 | PrimitiveNum::F64, &LitNum::Int { ref lit, neg }) => {
            let mag = match parse_u128_for_range(lit) {
                Ok(v) => v,
                Err(e) => {
                    compile_errors.push(e.to_compile_error());
                    return None;
                }
            };

            if matches!(prim, PrimitiveNum::F32) {
                let signed = if neg { -(mag as f64) } else { mag as f64 };
                if let Some(err) = check_float_fits_primitive(lit.span(), signed, prim) {
                    compile_errors.push(err);
                    return None;
                }
            }

            Some(emit_float_from_mag(lit.span(), mag, neg))
        }

        (PrimitiveNum::F32 | PrimitiveNum::F64, &LitNum::Float { ref lit, neg }) => {
            if matches!(prim, PrimitiveNum::F32) {
                match parse_f64_for_range(lit) {
                    Ok(v64) => {
                        let signed = if neg { -v64 } else { v64 };
                        if let Some(err) = check_float_fits_primitive(lit.span(), signed, prim) {
                            compile_errors.push(err);
                            return None;
                        }
                    }
                    Err(e) => {
                        compile_errors.push(e.to_compile_error());
                        return None;
                    }
                }
            }
            Some(emit_float_from_float(lit, neg))
        }
    }
}

/// Produces the RHS token for `#[validate(multiple_of)]` on integer fields,
/// emitting `compile_error!` for invalid use (non-numeric, float, zero, or out-of-range).
/// Returns `(rhs, Some(mask))` for unsigned powers of two (bitmask optimization),
/// `(rhs, None)` otherwise, or `None` if a compile-time error was emitted.
pub fn rhs_for_integer_multiple_of(
    prim: PrimitiveNum,
    lit: &LitInt,
    field_ident: &Ident,
    compile_errors: &mut Vec<TokenStream>,
) -> Option<(TokenStream, Option<syn::LitInt>)> {
    match prim {
        PrimitiveNum::F32 | PrimitiveNum::F64 => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(multiple_of)]` is not allowed on float fields");
            });
            None
        }
        PrimitiveNum::NonNumeric => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(multiple_of)]` can only be applied to integer fields");
            });
            None
        }
        _ => {
            let mag = match parse_u128_for_range(lit) {
                Ok(v) => v,
                Err(e) => {
                    compile_errors.push(e.to_compile_error());
                    return None;
                }
            };

            if mag == 0 {
                compile_errors.push(quote_spanned! { lit.span() =>
                    compile_error!("`multiple_of` must be non-zero");
                });
                return None;
            }

            if let Some(err) = check_int_fits_primitive(lit.span(), false, mag, prim) {
                compile_errors.push(err);
                return None;
            }

            let pow2_mask = if !is_signed(&prim) && (mag & (mag - 1)) == 0 {
                let mask = mag - 1;
                Some(syn::LitInt::new(&mask.to_string(), lit.span()))
            } else {
                None
            };

            Some((quote! { #lit }, pow2_mask))
        }
    }
}

/// Comparison operation used when validating numeric ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LitNumOperation {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Nq,
}

/// Compares two parsed numeric literals.
///
/// Both literals must have the same numeric category: integer or
/// floating-point.
///
/// Integer values are compared using their sign and `u128` magnitude. This
/// avoids narrowing large unsigned literals to `i128`.
///
/// # Errors
///
/// Returns an error when one literal is an integer and the other is a float.
pub fn compare_lit_num(
    left: &LitNum,
    operation: LitNumOperation,
    right: &LitNum,
    field_ident: &Ident,
) -> Result<bool> {
    match (left, right) {
        (
            LitNum::Int {
                lit: left_lit,
                neg: left_negative,
            },
            LitNum::Int {
                lit: right_lit,
                neg: right_negative,
            },
        ) => {
            let left_magnitude = parse_u128_for_range(left_lit)?;

            let right_magnitude = parse_u128_for_range(right_lit)?;

            let ordering = compare_signed_magnitudes(
                left_magnitude,
                *left_negative,
                right_magnitude,
                *right_negative,
            );

            Ok(operation_matches_ordering(operation, ordering))
        }

        (
            LitNum::Float {
                lit: left_lit,
                neg: left_negative,
            },
            LitNum::Float {
                lit: right_lit,
                neg: right_negative,
            },
        ) => {
            let left_magnitude = parse_f64_for_range(left_lit)?;

            let right_magnitude = parse_f64_for_range(right_lit)?;

            let left_value = if *left_negative {
                -left_magnitude
            } else {
                left_magnitude
            };

            let right_value = if *right_negative {
                -right_magnitude
            } else {
                right_magnitude
            };

            Ok(match operation {
                LitNumOperation::Gt => left_value > right_value,
                LitNumOperation::Lt => left_value < right_value,
                LitNumOperation::Gte => left_value >= right_value,
                LitNumOperation::Lte => left_value <= right_value,
                LitNumOperation::Eq => left_value == right_value,
                LitNumOperation::Nq => left_value != right_value,
            })
        }

        _ => Err(syn::Error::new(
            field_ident.span(),
            "`left` and `right` must have the same numeric type",
        )),
    }
}

fn compare_signed_magnitudes(
    left_magnitude: u128,
    left_negative: bool,
    right_magnitude: u128,
    right_negative: bool,
) -> Ordering {
    // Negative zero and positive zero represent the same value.
    let left_negative = left_negative && left_magnitude != 0;

    let right_negative = right_negative && right_magnitude != 0;

    match (left_negative, right_negative) {
        (true, true) => right_magnitude.cmp(&left_magnitude),

        (true, false) => Ordering::Less,

        (false, true) => Ordering::Greater,

        (false, false) => left_magnitude.cmp(&right_magnitude),
    }
}

fn operation_matches_ordering(operation: LitNumOperation, ordering: Ordering) -> bool {
    match operation {
        LitNumOperation::Gt => ordering.is_gt(),
        LitNumOperation::Lt => ordering.is_lt(),
        LitNumOperation::Gte => ordering.is_gt() || ordering.is_eq(),
        LitNumOperation::Lte => ordering.is_lt() || ordering.is_eq(),
        LitNumOperation::Eq => ordering.is_eq(),
        LitNumOperation::Nq => !ordering.is_eq(),
    }
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, format_ident};
    use syn::{LitInt, parse_quote};

    use crate::validate::args::{LitNum, PrimitiveNum};

    use super::{
        LitNumOperation, compare_lit_num, rhs_for_integer_multiple_of, rhs_for_numeric_bound,
    };

    #[test]
    fn integer_bound_emits_integer_expression() {
        let field = format_ident!("age");

        let bound = LitNum::Int {
            lit: parse_quote!(42),
            neg: false,
        };

        let mut compile_errors = Vec::new();

        let rhs = rhs_for_numeric_bound(PrimitiveNum::I32, &bound, &field, &mut compile_errors)
            .expect("integer bound should generate");

        assert_eq!(compact(&rhs), "42");
        assert!(compile_errors.is_empty());
    }

    #[test]
    fn integer_bound_for_float_emits_float_expression() {
        let field = format_ident!("score");

        let bound = LitNum::Int {
            lit: parse_quote!(42),
            neg: false,
        };

        let mut compile_errors = Vec::new();

        let rhs = rhs_for_numeric_bound(PrimitiveNum::F64, &bound, &field, &mut compile_errors)
            .expect("integer bound should convert to float");

        assert_eq!(compact(&rhs), "42.0");
        assert!(compile_errors.is_empty());
    }

    #[test]
    fn negative_bound_is_rejected_for_unsigned_field() {
        let field = format_ident!("count");

        let bound = LitNum::Int {
            lit: parse_quote!(1),
            neg: true,
        };

        let mut compile_errors = Vec::new();

        let rhs = rhs_for_numeric_bound(PrimitiveNum::U8, &bound, &field, &mut compile_errors);

        assert!(rhs.is_none());
        assert_eq!(compile_errors.len(), 1);

        let error = compact(&compile_errors[0]);

        assert!(
            error.contains("numericbounddoesnotfit`u8`"),
            "unexpected compile error: {error}"
        );
    }

    #[test]
    fn float_bound_is_rejected_for_integer_field() {
        let field = format_ident!("count");

        let bound = LitNum::Float {
            lit: parse_quote!(2.5),
            neg: false,
        };

        let mut compile_errors = Vec::new();

        let rhs = rhs_for_numeric_bound(PrimitiveNum::I32, &bound, &field, &mut compile_errors);

        assert!(rhs.is_none());
        assert_eq!(compile_errors.len(), 1);

        let error = compact(&compile_errors[0]);

        assert!(
            error.contains("floatliteralisnotallowedforintegerfield"),
            "unexpected compile error: {error}"
        );
    }

    #[test]
    fn unsigned_power_of_two_generates_bitmask() {
        let field = format_ident!("count");
        let literal: LitInt = parse_quote!(8);
        let mut compile_errors = Vec::new();

        let (rhs, mask) =
            rhs_for_integer_multiple_of(PrimitiveNum::U32, &literal, &field, &mut compile_errors)
                .expect("multiple_of should generate");

        assert_eq!(compact(&rhs), "8");

        assert_eq!(
            mask.expect("power of two should generate mask")
                .base10_digits(),
            "7"
        );

        assert!(compile_errors.is_empty());
    }

    #[test]
    fn signed_power_of_two_does_not_generate_bitmask() {
        let field = format_ident!("count");
        let literal: LitInt = parse_quote!(8);
        let mut compile_errors = Vec::new();

        let (rhs, mask) =
            rhs_for_integer_multiple_of(PrimitiveNum::I32, &literal, &field, &mut compile_errors)
                .expect("multiple_of should generate");

        assert_eq!(compact(&rhs), "8");
        assert!(mask.is_none());
        assert!(compile_errors.is_empty());
    }

    #[test]
    fn compares_integer_literals_without_narrowing_to_i128() {
        let field = format_ident!("value");

        let maximum_u128 = LitNum::Int {
            lit: parse_quote!(340282366920938463463374607431768211455),
            neg: false,
        };

        let maximum_i128 = LitNum::Int {
            lit: parse_quote!(170141183460469231731687303715884105727),
            neg: false,
        };

        assert!(
            compare_lit_num(&maximum_u128, LitNumOperation::Gt, &maximum_i128, &field,)
                .expect("integer comparison should succeed")
        );
    }

    #[test]
    fn compares_negative_integer_magnitudes_correctly() {
        let field = format_ident!("value");

        let negative_ten = LitNum::Int {
            lit: parse_quote!(10),
            neg: true,
        };

        let negative_one = LitNum::Int {
            lit: parse_quote!(1),
            neg: true,
        };

        assert!(
            compare_lit_num(&negative_ten, LitNumOperation::Lt, &negative_one, &field,)
                .expect("negative comparison should succeed")
        );
    }

    #[test]
    fn negative_zero_equals_positive_zero() {
        let field = format_ident!("value");

        let negative_zero = LitNum::Int {
            lit: parse_quote!(0),
            neg: true,
        };

        let positive_zero = LitNum::Int {
            lit: parse_quote!(0),
            neg: false,
        };

        assert!(
            compare_lit_num(&negative_zero, LitNumOperation::Eq, &positive_zero, &field,)
                .expect("zero comparison should succeed")
        );
    }

    #[test]
    fn compares_float_literals() {
        let field = format_ident!("score");

        let left = LitNum::Float {
            lit: parse_quote!(2.5),
            neg: false,
        };

        let right = LitNum::Float {
            lit: parse_quote!(1.5),
            neg: false,
        };

        assert!(
            compare_lit_num(&left, LitNumOperation::Gt, &right, &field,)
                .expect("float comparison should succeed")
        );
    }

    #[test]
    fn mixed_numeric_literal_categories_are_rejected() {
        let field = format_ident!("value");

        let integer = LitNum::Int {
            lit: parse_quote!(1),
            neg: false,
        };

        let float = LitNum::Float {
            lit: parse_quote!(1.0),
            neg: false,
        };

        let error = compare_lit_num(&integer, LitNumOperation::Eq, &float, &field)
            .expect_err("mixed literal categories should fail");

        assert_eq!(
            error.to_string(),
            "`left` and `right` must have the same numeric type"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
