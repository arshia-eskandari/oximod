use crate::{
    index::IndexArgs,
    validate::{LitNum, ValidateArgs},
};
use proc_macro2::TokenStream;
use std::fmt::Display;
use std::str::FromStr;
use syn::{
    Attribute, Expr, ExprLit, ExprUnary, GenericArgument, Lit, LitFloat, LitInt, Meta,
    PathArguments, Type, UnOp, spanned::Spanned,
};

/// Parses and returns the expression provided as the argument to the given attribute.
pub fn parse_default_expr(attr: &Attribute) -> syn::Result<Expr> {
    let expr: Expr = attr.parse_args()?;
    Ok(expr)
}

/// If `ty` is `Option<Inner>`, returns `Some(&Inner)`, otherwise `None`.
pub fn option_inner_type(ty: &Type) -> Option<&Type> {
    // We only care about a simple `Option<...>` path type
    if let Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
    {
        let segment = &type_path.path.segments[0];
        if segment.ident == "Option"
            && let PathArguments::AngleBracketed(params) = &segment.arguments
            && params.args.len() == 1
            && let GenericArgument::Type(inner_ty) = &params.args[0]
        {
            return Some(inner_ty);
        }
    }
    None
}

/// Parse an expression that is a numeric literal, optionally with a leading `-`.
/// Accepts: `42`, `-42`, `3.14`, `-3.14`.
fn parse_num_lit_expr(expr: Expr) -> syn::Result<LitNum> {
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
fn litint_strictly_positive(lit: &syn::LitInt) -> syn::Result<()> {
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

/// Usage:
/// parse_lit_to_primitive_type!(lit, i32)              -> Result<i32,  syn::Error>
/// parse_lit_to_primitive_type!(&lit, String, "msg")   -> Result<String, syn::Error>
macro_rules! parse_lit_to_primitive_type {
    // 3-arg form with custom error message
    ($lit:expr, $ty:ty, $msg:expr) => {{
        // Support passing either `Lit` or `&Lit`
        let __lit: &::syn::Lit = &$lit;

        match __lit {
            ::syn::Lit::Str(s) => s.value().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(s), format!("{}: {}", $msg, e))
            }),
            ::syn::Lit::Int(i) => i.base10_digits().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(i), format!("{}: {}", $msg, e))
            }),
            ::syn::Lit::Float(f) => f.base10_digits().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(f), format!("{}: {}", $msg, e))
            }),
            other => ::core::result::Result::Err(::syn::Error::new(
                ::syn::spanned::Spanned::span(other),
                $msg,
            )),
        }
    }};

    // 2-arg form with a default message
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

/// Extracts and returns the literal value from an attribute, supporting both `#[attr = <lit>]`
/// and `#[attr(<lit>)]` forms.
pub fn lit_from_attr(attr: &Attribute) -> syn::Result<Lit> {
    match &attr.meta {
        Meta::NameValue(nv) => {
            if let Expr::Lit(expr_lit) = &nv.value {
                Ok(expr_lit.lit.clone())
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

/// Extract a single literal from `attr` and parse it into `T` using your
/// `parse_lit_to_primitive_type!` macro. If `msg` is `Some`, it's forwarded
/// to the macro; otherwise the macro's default message is used.
///
/// Works for `String` and numeric primitives (u*/i*/f*), i.e., any `T` that
/// implements `FromStr`.
fn parse_attr_value<T>(attr: &Attribute, msg: Option<&str>) -> syn::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    let lit: Lit = lit_from_attr(attr)?;
    match msg {
        Some(m) => parse_lit_to_primitive_type!(&lit, T, m),
        None => parse_lit_to_primitive_type!(&lit, T),
    }
}

/// Parses the key-value pairs and flags provided to an `#[index(...)]` attribute into an `IndexArgs` struct.
pub fn parse_index_args(attr: &Attribute) -> syn::Result<IndexArgs> {
    let mut args = IndexArgs::default();

    if attr.path().is_ident("index") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("unique") {
                args.unique = Some(true);
            } else if meta.path.is_ident("sparse") {
                args.sparse = Some(true);
            } else if meta.path.is_ident("background") {
                args.background = Some(true);
            } else if meta.path.is_ident("name") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.name = Some(lit_str.value());
                }
            } else if meta.path.is_ident("order") {
                let lit: Lit = meta.value()?.parse()?;
                let order_val: i32 = parse_lit_to_primitive_type!(lit, i32)?;
                args.order = Some(order_val);
            } else if meta.path.is_ident("expire_after_secs") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.expire_after_secs = Some(lit_int.base10_parse::<i32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `expire_after_secs`",
                    ));
                }
            } else if meta.path.is_ident("version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_primitive_type!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`version` must be greater than or equal to 1",
                    ));
                }
                args.version = Some(version);
            } else if meta.path.is_ident("text_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let text_index_version = parse_lit_to_primitive_type!(&lit, u32)?;
                if text_index_version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`text_index_version` must be greater than or equal to 1",
                    ));
                }
                args.text_index_version = Some(text_index_version);
            } else if meta.path.is_ident("hidden") {
                args.hidden = Some(true);
            }

            Ok(())
        })?;
    }

    Ok(args)
}

/// If the provided type is `Option<Inner>`, returns a reference to the inner type; otherwise returns `None`.
pub fn unwrap_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(generic_args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = generic_args.args.first()
    {
        return Some(inner_ty);
    }
    None
}

/// Parses the key-value pairs and flags provided to a `#[validate(...)]` attribute into a `ValidateArgs` struc.
pub fn parse_validate_args(attr: &Attribute) -> syn::Result<ValidateArgs> {
    let mut args = ValidateArgs::default();

    if attr.path().is_ident("validate") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("min_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `min_length`",
                    ));
                }
            } else if meta.path.is_ident("max_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `max_length`",
                    ));
                }
            } else if meta.path.is_ident("required") {
                args.required = true;
            } else if meta.path.is_ident("email") {
                args.email = true;
            } else if meta.path.is_ident("pattern") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.pattern = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `pattern`",
                    ));
                }
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
                let expr: Expr = meta.value()?.parse()?;
                args.min = Some(parse_num_lit_expr(expr)?);
            } else if meta.path.is_ident("max") {
                let expr: Expr = meta.value()?.parse()?;
                args.max = Some(parse_num_lit_expr(expr)?);
            } else if meta.path.is_ident("starts_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.starts_with = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `starts_with`",
                    ));
                }
            } else if meta.path.is_ident("ends_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.ends_with = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `ends_with`",
                    ));
                }
            } else if meta.path.is_ident("includes") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.includes = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `includes`",
                    ));
                }
            } else if meta.path.is_ident("alphanumeric") {
                args.alphanumeric = true;
            } else if meta.path.is_ident("multiple_of") {
                let lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    litint_strictly_positive(&lit_int)?;
                    args.multiple_of = Some(lit_int);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `multiple_of`",
                    ));
                }
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(args)
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

/// Parses a single-value attribute into type `T` and converts any parse errors
/// into a compile error token stream.
pub fn parse_attr_value_ts<T>(attr: &Attribute, msg: Option<&str>) -> Result<T, TokenStream>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    parse_attr_value::<T>(attr, msg).map_err(|e| e.to_compile_error())
}
