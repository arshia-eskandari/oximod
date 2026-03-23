use super::super::args::{BuiltChecks, ValidateArgs};
use quote::quote;

pub fn build_length_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_name_lit: &syn::LitStr,
    is_str: bool,
) {
    if let Some(min_length) = validate_args.min_length {
        build_checks.field_rules_val.push(quote! {
            if val.len() < (#min_length as usize) {
                return Err(
                    ::oximod::OxiModError::validation(
                        format!(
                            "Field '{}' must have a length of at least {}",
                            #field_name_lit,
                            #min_length
                        )
                    )
                );
            }
        });
    }

    if let Some(max_length) = validate_args.max_length {
        build_checks.field_rules_val.push(quote! {
            if val.len() > (#max_length as usize) {
                return Err(
                    ::oximod::OxiModError::validation(
                        format!(
                            "Field '{}' must have a length of at most {}",
                            #field_name_lit,
                            #max_length
                        )
                    )
                );
            }
        });
    }

    if validate_args.non_empty {
        let condition = if is_str {
            quote! { val.trim().is_empty() }
        } else {
            quote! { val.len() == 0 }
        };
        build_checks.field_rules_val.push(quote! {
            if #condition {
                return Err(
                    ::oximod::OxiModError::validation(
                        format!("Field '{}' must be non-empty", #field_name_lit)
                    )
                );
            }
        });
    }
}
