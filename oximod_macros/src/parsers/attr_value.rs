use crate::parsers::macros::parse_lit_to_primitive_type;
use proc_macro2::TokenStream;
use std::fmt::Display;
use std::str::FromStr;
use syn::{Attribute, Expr, Lit, Meta, spanned::Spanned};

/// Extracts and returns the literal value from an attribute, supporting both `#[attr = <lit>]`
/// and `#[attr(<lit>)]` forms.
fn lit_from_attr(attr: &Attribute) -> syn::Result<Lit> {
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

/// Parses a single-value attribute into type `T` and converts any parse errors
/// into a compile error token stream.
pub fn parse_attr_value_ts<T>(attr: &Attribute, msg: Option<&str>) -> Result<T, TokenStream>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    parse_attr_value::<T>(attr, msg).map_err(|e| e.to_compile_error())
}
