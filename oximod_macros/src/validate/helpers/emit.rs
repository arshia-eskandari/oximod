use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{LitFloat, LitInt};

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
pub fn emit_float_from_mag(span: Span, mag: u128, neg: bool) -> TokenStream {
    // decimal string regardless of original literal radix; add ".0"
    let s = if neg {
        format!("-{mag}.0")
    } else {
        format!("{mag}.0")
    };
    let lf = LitFloat::new(&s, span);
    quote! { #lf }
}
