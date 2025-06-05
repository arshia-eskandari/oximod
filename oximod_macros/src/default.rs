use proc_macro2::TokenStream;
use quote::quote;
use syn::{ Attribute, GenericArgument, LitStr, PathArguments, Type, Ident };
pub struct DefaultDefinition {
    pub field_ident: syn::Ident,
    pub default_expr: proc_macro2::TokenStream,
}

pub fn parse_default_args(
    attr: &Attribute,
    field_ident: &syn::Ident
) -> syn::Result<DefaultDefinition> {
    // Accept #[default(Status::Pending)] or #[default(42 + 5)]
    let expr: syn::Expr = attr.parse_args()?;
    Ok(DefaultDefinition {
        field_ident: field_ident.clone(),
        default_expr: quote! { #expr },
    })
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

pub fn maybe_push_id_setter(
    has_id_attr: bool,
    input_attrs: &[Attribute],
    setters: &mut Vec<TokenStream>
) {
    if has_id_attr {
        let mut id_setter_name = "id".to_string();

        for attr in input_attrs {
            if attr.path().is_ident("document_id_setter_ident") {
                let setter_lit: LitStr = attr
                    .parse_args()
                    .expect("Expected #[document_id_setter_ident(\"...\")]");
                id_setter_name = setter_lit.value();
            }
        }

        let id_method_ident = syn::Ident::new(&id_setter_name, proc_macro2::Span::call_site());
        let id_setter =
            quote! {
                /// Set the MongoDB ObjectId
                pub fn #id_method_ident(mut self, id: ::oximod::_mongodb::bson::oid::ObjectId) -> Self {
                    self._id = Some(id);
                    self
                }
            };
        setters.push(id_setter);
    }
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
