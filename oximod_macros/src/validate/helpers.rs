use crate::parsers::{parse_f64_for_range, parse_u128_for_range};
use crate::validate::args::{LitNum, PrimitiveNum};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Ident, LitFloat, LitInt, Type};

#[inline]
pub fn is_signed(prim: PrimitiveNum) -> bool {
    use PrimitiveNum::*;
    matches!(prim, I8 | I16 | I32 | I64 | I128 | Isize | F32 | F64)
}

#[inline]
pub fn is_integer(prim: PrimitiveNum) -> bool {
    use PrimitiveNum::*;
    matches!(
        prim,
        I8 | I16 | I32 | I64 | I128 | Isize | U8 | U16 | U32 | U64 | U128 | Usize
    )
}

#[inline]
pub fn is_string(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => tp.path.is_ident("String")
            || tp.path.is_ident("Cow") // Cow<'_, str>
                && tp.path.segments.last().is_some_and(|seg| {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        ab.args.iter().any(|arg| matches!(arg,
                            syn::GenericArgument::Type(syn::Type::Path(p)) if p.path.is_ident("str")))
                    } else { false }
                }),
        Type::Reference(r) => match &*r.elem {
            Type::Path(tp) => tp.path.is_ident("str") || tp.path.is_ident("String"),
            _ => false,
        },
        _ => false,
    }
}

#[inline]
pub fn is_numeric(prim: &PrimitiveNum) -> bool {
    prim != &PrimitiveNum::NonNumeric
}

#[inline]
pub fn primitive_of(ty: &syn::Type) -> PrimitiveNum {
    use PrimitiveNum::*;
    let inner = crate::parsers::unwrap_option_type(ty).unwrap_or(ty);

    let syn::Type::Path(tp) = inner else {
        return NonNumeric;
    };
    let Some(seg) = tp.path.segments.last() else {
        return NonNumeric;
    };
    let id = &seg.ident;

    if id == "i8" {
        I8
    } else if id == "i16" {
        I16
    } else if id == "i32" {
        I32
    } else if id == "i64" {
        I64
    } else if id == "i128" {
        I128
    } else if id == "isize" {
        Isize
    } else if id == "u8" {
        U8
    } else if id == "u16" {
        U16
    } else if id == "u32" {
        U32
    } else if id == "u64" {
        U64
    } else if id == "u128" {
        U128
    } else if id == "usize" {
        Usize
    } else if id == "f32" {
        F32
    } else if id == "f64" {
        F64
    } else {
        NonNumeric
    }
}

#[inline]
pub fn emit_int_lit(lit: &LitInt, neg: bool) -> TokenStream {
    if neg {
        quote! { - #lit }
    } else {
        quote! { #lit }
    }
}

#[inline]
pub fn emit_float_from_float(lit: &LitFloat, neg: bool) -> TokenStream {
    if neg {
        quote! { - #lit }
    } else {
        quote! { #lit }
    }
}

#[inline]
fn emit_float_from_mag(span: Span, mag: u128, neg: bool) -> TokenStream {
    // decimal string regardless of original literal radix; add ".0"
    let s = if neg {
        format!("-{mag}.0")
    } else {
        format!("{mag}.0")
    };
    let lf = LitFloat::new(&s, span);
    quote! { #lf }
}

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
            &LitNum::Float { lit: _, .. },
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

            let pow2_mask = if !is_signed(prim) && (mag & (mag - 1)) == 0 {
                let mask = mag - 1;
                Some(syn::LitInt::new(&mask.to_string(), lit.span()))
            } else {
                None
            };

            Some((quote! { #lit }, pow2_mask))
        }
    }
}
