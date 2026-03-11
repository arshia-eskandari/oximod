use super::super::args::{BuiltChecks, ValidateArgs};
use quote::quote;
use syn::Ident;

pub fn build_option_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
) {
    if let Some(true) = validate_args.required {
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
