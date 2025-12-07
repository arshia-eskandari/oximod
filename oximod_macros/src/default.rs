use crate::parsers::option_inner_type;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

/// Appends a setter method for the `_id` field if it exists, using the provided
/// setter name and accepting a MongoDB `ObjectId`.
pub fn push_id_setter(
    setters: &mut Vec<TokenStream>,
    id_setter_name: &str,
) -> Result<(), TokenStream> {
    let id_method_ident = syn::Ident::new(&id_setter_name, proc_macro2::Span::call_site());
    let id_setter = quote! {
        /// Set the MongoDB ObjectId
        pub fn #id_method_ident(mut self, id: ::oximod::_mongodb::bson::oid::ObjectId) -> Self {
            self._id = Some(id);
            self
        }
    };
    setters.push(id_setter);

    Ok(())
}

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
