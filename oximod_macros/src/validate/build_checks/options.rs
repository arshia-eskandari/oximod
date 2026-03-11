use super::super::args::BuiltChecks;
use quote::{quote, quote_spanned};
use syn::Ident;

pub fn build_option_checks(
    build_checks: &mut BuiltChecks,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
    is_optional: bool,
) {
    if !is_optional {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' cannot use #[validate(required)] because it is not Option<T>"
                    )
                );
            });
    } else {
        build_checks.field_rules_direct.push(quote! {
            if self.#field_ident.is_none() {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' is required", #field_name_lit)
                    )
                );
            }
        });
    }
}
