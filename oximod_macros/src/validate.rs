pub mod args;
mod build_checks;
pub mod helpers;
pub mod macros;

use crate::parsers::unwrap_option_type;
pub use args::{BuiltChecks, LitNum, ValidateArgs};
use build_checks::{
    numbers::{build_integer_checks, build_number_checks, build_signed_number_checks},
    options::build_option_checks,
    strings::build_string_checks,
};
use helpers::{is_integer, is_numeric, is_signed, is_string, primitive_of};
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
    let inner_ty = opt_inner.unwrap_or(field_ty);
    let prim = primitive_of(inner_ty);

    if validate_args.has_type_collision() {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                "invalid validation rules: cannot apply validations from different type groups to the same field"
            );
        }];
    }

    if validate_args.must_be_number() && !is_numeric(&prim) {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                concat!(
                    "Field '", stringify!(#field_ident),
                    "' uses numeric validation rules, but its type is not numeric"
                )
            );
        }];
    } else {
        build_number_checks(
            &mut build_checks,
            &validate_args,
            prim,
            field_ident,
            &field_name_lit,
        );
    }

    if validate_args.must_be_signed_number() && !is_signed(&prim) {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                concat!(
                    "Field '", stringify!(#field_ident),
                    "' uses signed-number validation rules, but its type is not a signed numeric type"
                )
            );
        }];
    } else {
        build_signed_number_checks(&mut build_checks, &validate_args, &field_name_lit);
    }

    if validate_args.must_be_integer() && !is_integer(&prim) {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                concat!(
                    "Field '", stringify!(#field_ident),
                    "' uses integer-only validation rules, but its type is not an integer"
                )
            );
        }];
    } else {
        build_integer_checks(
            &mut build_checks,
            &validate_args,
            prim,
            field_ident,
            &field_name_lit,
        );
    }

    if validate_args.must_be_string() && !is_string(inner_ty) {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                concat!(
                    "Field '", stringify!(#field_ident),
                    "' uses string validation rules, but its type is not String or &str"
                )
            );
        }];
    } else {
        build_string_checks(
            &mut build_checks,
            &validate_args,
            field_ident,
            &field_name_lit,
            struct_ident,
        );
    }

    if validate_args.must_be_optional() {
        build_option_checks(
            &mut build_checks,
            field_ident,
            &field_name_lit,
            opt_inner.is_some(),
        );
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
        let grouped = opt_check!(opt_inner.is_some(), field_ident, {
            #(#field_rules_val)*
        });
        build_checks.checks.push(grouped);
    }

    build_checks.checks
}
