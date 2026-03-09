pub mod args;
mod build;
pub mod helpers;
pub mod macros;

use crate::parsers::unwrap_option_type;
pub use args::{BuiltChecks, LitNum, ValidateArgs};
use build::{numeric::build_numeric_checks, string::build_string_checks};
use helpers::{is_numeric, is_signed, is_string, primitive_of};
use macros::opt_check;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Ident, Type};

/// Generates validation `TokenStream`s for a field based on `ValidateArgs`,
/// producing compile-time and runtime checks appropriate to the field’s type.
pub fn generate_validate_model_tokens(
    struct_ident: &Ident,
    field_ident: &Ident,
    field_ty: &Type,
    validate_args: ValidateArgs,
) -> Vec<TokenStream> {
    let mut build_checks = BuiltChecks::default();
    let field_name_lit = syn::LitStr::new(&field_ident.to_string(), field_ident.span());
    let opt_inner = unwrap_option_type(field_ty);
    let is_optional = opt_inner.is_some();
    let inner_ty = opt_inner.unwrap_or(field_ty);
    let is_str = is_string(inner_ty);
    let prim = primitive_of(inner_ty);
    let is_num = is_numeric(&prim);

    build_string_checks(
        &mut build_checks,
        &validate_args,
        is_str,
        field_ident,
        &field_name_lit,
        struct_ident,
    );
    build_numeric_checks(
        &mut build_checks,
        &validate_args,
        is_num,
        prim,
        field_ident,
        &field_name_lit,
    );

    if matches!(validate_args.required, Some(true)) {
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

    if !build_checks.numeric_checks.is_empty() {
        let numeric_checks = build_checks.numeric_checks;
        build_checks.field_rules_val.push(quote! {
            let v = *val;
            #(#numeric_checks)*
        });
    }
    build_checks.checks.extend(build_checks.compile_errors);

    if !build_checks.field_rules_direct.is_empty() {
        let field_rules_direct = build_checks.field_rules_direct;
        build_checks
            .checks
            .push(quote! { { #(#field_rules_direct)* } });
    }

    if !build_checks.field_rules_val.is_empty() {
        let field_rules_val = build_checks.field_rules_val;
        let grouped = opt_check!(is_optional, field_ident, {
            #(#field_rules_val)*
        });
        build_checks.checks.push(grouped);
    }

    build_checks.checks
}
