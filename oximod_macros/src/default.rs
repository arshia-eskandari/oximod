use quote::quote;
use syn::{ Attribute, Type, PathArguments, GenericArgument };
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
