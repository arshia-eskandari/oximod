use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, GenericArgument, Ident, PathArguments, Type};

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
