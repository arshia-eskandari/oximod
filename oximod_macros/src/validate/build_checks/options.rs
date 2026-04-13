use super::super::args::{BuiltChecks, ValidateArgs};
use quote::quote;
use syn::Ident;

pub fn build_option_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
) {
    if validate_args.required {
        build_checks.field_rules_direct.push(quote! {
            if self.#field_ident.is_none() {
                validation_errors.push(::oximod::ValidationError::new(
                    #field_name_lit,
                    "is required",
                ));
            }
        });
    }
}
