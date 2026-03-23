use crate::parsers::option_inner_type;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

/// Appends setter method for all non-`_id` fields, generating `Option`-aware
/// setters for optional types and direct setters for non-optional types.
pub fn push_field_setter(ident: &Ident, ty: &Type, setters: &mut Vec<TokenStream>) {
    let setter = if let Some(inner) = option_inner_type(ty) {
        quote! {
            pub fn #ident<T>(mut self, val: T) -> Self
            where
                T: Into<#inner>,
            {
                self.#ident = Some(val.into());
                self
            }
        }
    } else {
        quote! {
            pub fn #ident<T>(mut self, val: T) -> Self
            where
                T: Into<#ty>,
            {
                self.#ident = val.into();
                self
            }
        }
    };

    setters.push(setter);
}
