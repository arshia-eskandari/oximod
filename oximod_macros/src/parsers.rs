use crate::{index::IndexArgs, validate::ValidateArgs};
use std::fmt::Display;
use std::str::FromStr;
use syn::{spanned::Spanned, Attribute, Expr, GenericArgument, Lit, Meta, PathArguments, Type};

/// Parses and returns the expression provided as the argument to the given attribute.
pub fn parse_default_expr(attr: &Attribute) -> syn::Result<Expr> {
    let expr: Expr = attr.parse_args()?;
    Ok(expr)
}

/// If `ty` is `Option<Inner>`, returns `Some(&Inner)`, otherwise `None`.
pub fn option_inner_type(ty: &Type) -> Option<&Type> {
    // We only care about a simple `Option<...>` path type
    if let Type::Path(type_path) = ty {
        // Must be exactly one segment, i.e. `Option`
        if type_path.path.segments.len() == 1 {
            let segment = &type_path.path.segments[0];
            if segment.ident == "Option" {
                // Look for the angle-bracketed args: `<Inner>`
                if let PathArguments::AngleBracketed(params) = &segment.arguments {
                    // We expect exactly one generic argument
                    if params.args.len() == 1 {
                        // And that argument must itself be a type
                        if let GenericArgument::Type(inner_ty) = &params.args[0] {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    None
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
pub(crate) use parse_lit_to_primitive_type;

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
pub fn parse_attr_value<T>(attr: &Attribute, msg: Option<&str>) -> syn::Result<T>
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
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = generic_args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
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
                args.required = Some(true);
            } else if meta.path.is_ident("email") {
                args.email = Some(true);
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
                args.non_empty = Some(true);
            } else if meta.path.is_ident("positive") {
                args.positive = Some(true);
            } else if meta.path.is_ident("negative") {
                args.negative = Some(true);
            } else if meta.path.is_ident("non_negative") {
                args.non_negative = Some(true);
            } else if meta.path.is_ident("min") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `min`",
                    ));
                }
            } else if meta.path.is_ident("max") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `max`",
                    ));
                }
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
                args.alphanumeric = Some(true);
            } else if meta.path.is_ident("multiple_of") {
                let lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = &lit {
                    let val = lit_int.base10_parse::<i64>()?;
                    if val == 0 {
                        return Err(syn::Error::new(
                            lit.span(),
                            "`multiple_of` must be greater than 0",
                        ));
                    }
                    args.multiple_of = Some(val);
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
