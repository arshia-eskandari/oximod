use crate::parsers::option_inner_type;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

/// Appends a setter method for the `_id` field if it exists, using the provided
/// setter name and accepting a MongoDB `ObjectId`.
pub fn push_id_setter(
    has_id_attr: bool,
    setters: &mut Vec<TokenStream>,
    id_setter_name: String,
) -> Result<(), TokenStream> {
    if has_id_attr {
        let id_method_ident = syn::Ident::new(&id_setter_name, proc_macro2::Span::call_site());
        let id_setter = quote! {
            /// Set the MongoDB ObjectId
            pub fn #id_method_ident(mut self, id: ::oximod::_mongodb::bson::oid::ObjectId) -> Self {
                self._id = Some(id);
                self
            }
        };
        setters.push(id_setter);
    }

    Ok(())
}

/// Appends setter methods for all non-`_id` fields, generating `Option`-aware
/// setters for optional types and direct setters for non-optional types.
pub fn push_field_setters(all_fields: &[(Ident, Type)], setters: &mut Vec<TokenStream>) {
    for (ident, ty) in all_fields.iter().filter(|(ident, _)| ident != "_id") {
        let setter = if let Some(inner) = option_inner_type(ty) {
            quote! {
                pub fn #ident<T: Into<#inner>>(mut self, val: T) -> Self {
                    self.#ident = Some(val.into());
                    self
                }
            }
        } else {
            quote! {
                pub fn #ident(mut self, val: #ty) -> Self {
                    self.#ident = val;
                    self
                }
            }
        };
        setters.push(setter);
    }
}
