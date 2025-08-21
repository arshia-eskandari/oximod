pub mod args;

mod helpers;
mod macros;

pub use args::{LitNum, ValidateArgs};

use crate::parsers::unwrap_option_type;
use helpers::{
    is_integer, is_numeric, is_signed, is_string, primitive_of, rhs_for_integer_multiple_of,
    rhs_for_numeric_bound,
};
use macros::{is_type_safe, opt_check};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, Type};

/// Generates validation `TokenStream`s for a field based on `ValidateArgs`,
/// producing compile-time and runtime checks appropriate to the field’s type.
pub fn generate_validate_model_tokens(
    struct_ident: &Ident,
    field_ident: &Ident,
    field_ty: &Type,
    validate_args: ValidateArgs,
) -> Vec<TokenStream> {
    let ValidateArgs {
        min_length: min_length_option,
        max_length: max_length_option,
        required,
        email: email_option,
        pattern: pattern_option,
        non_empty: non_empty_option,
        positive: positive_option,
        negative: negative_option,
        non_negative: non_negative_option,
        min: min_option,
        max: max_option,
        starts_with: starts_with_option,
        ends_with: ends_with_option,
        includes: includes_option,
        alphanumeric: alphanumeric_option,
        multiple_of: multiple_of_option,
    } = &validate_args;
    let field_name_str = field_ident.to_string();

    let opt_inner = unwrap_option_type(field_ty);
    let is_optional = opt_inner.is_some();
    let inner_ty = opt_inner.unwrap_or(field_ty);

    let mut checks: Vec<TokenStream> = Vec::new();

    let mut compile_errors: Vec<TokenStream> = Vec::new();
    let mut field_rules_val: Vec<TokenStream> = Vec::new();
    let mut field_rules_direct: Vec<TokenStream> = Vec::new();

    let is_str = is_string(inner_ty);
    let prim = primitive_of(inner_ty);
    let is_num = is_numeric(&prim);

    if matches!(required, Some(true)) {
        if !is_optional {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' cannot use #[validate(required)] because it is not Option<T>"
                    )
                );
            });
        } else {
            field_rules_direct.push(quote! {
                if self.#field_ident.is_none() {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' is required", #field_name_str)
                        ),
                        concat!("Provide a value for '", stringify!(#field_ident), "'")
                    ));
                }
            });
        }
    }

    if let Some(min_length) = min_length_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(min_length)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.len() < (#min_length as usize) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!(
                                "Field '{}' must be at least {} characters long",
                                #field_name_str,
                                #min_length
                            )
                        ),
                        &format!(
                            "Ensure '{}' has at least {} characters", stringify!(#field_ident), #min_length
                        )
                    ));
                }
            });
        }
    }

    if let Some(max_length) = max_length_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(max_length)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.len() > (#max_length as usize) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!(
                                "Field '{}' must be at most {} characters long",
                                #field_name_str,
                                #max_length
                            )
                        ),
                        &format!(
                            "Ensure '{}' has at most {} characters", stringify!(#field_ident), #max_length
                        )
                    ));
                }
            });
        }
    }

    if matches!(email_option, Some(true))
        && is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(email)]` can only be applied to string fields"
        )
    {
        field_rules_val.push(quote! {
            match val.split_once('@') {
                Some((local, domain)) if !local.is_empty() && !domain.is_empty() => {
                    match domain.rsplit_once('.') {
                        Some((lhs, rhs)) if !lhs.is_empty() && !rhs.is_empty() => { /* OK */ }
                        _ => {
                            return Err(::oximod::_attach_printables!(
                                ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                    format!("Field '{}' must be a valid email address", #field_name_str)
                                ),
                                concat!("Ensure '", stringify!(#field_ident), "' is in the format local@domain")
                            ));
                        }
                    }
                }
                _ => {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be a valid email address", #field_name_str)
                        ),
                        concat!("Ensure '", stringify!(#field_ident), "' is in the format local@domain")
                    ));
                }
            }
        });
    }

    if let Some(pattern) = pattern_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(pattern)]` can only be applied to string fields"
        ) {
            if let Err(e) = ::regex::Regex::new(pattern) {
                let msg =
                    format!("Invalid regex pattern in validation for '{field_name_str}': {e}");
                compile_errors.push(quote_spanned! { field_ident.span() =>
                    compile_error!(#msg);
                });
            } else {
                let pattern_lit = syn::LitStr::new(pattern, field_ident.span());

                let re_ident = format_ident!("__oximod_re_{}_{}", struct_ident, field_ident);

                field_rules_val.push(quote! {
                    #[allow(non_upper_case_globals)]
                    static #re_ident: ::std::sync::OnceLock<::oximod::_regex::Regex> =
                        ::std::sync::OnceLock::new();

                    let regex = #re_ident
                        .get_or_init(|| ::oximod::_regex::Regex::new(#pattern_lit).unwrap());

                    if !regex.is_match(val) {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!(
                                    "Field '{}' does not match the required pattern",
                                    #field_name_str
                                )
                            ),
                            concat!(
                                "Ensure '", stringify!(#field_ident),
                                "' matches regex ",
                            )
                        ));
                    }
                });
            }
        }
    }

    if let Some(true) = non_empty_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(non_empty)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.trim().is_empty() {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be non-empty", #field_name_str)
                        ),
                        concat!("Provide a non-empty string for '", stringify!(#field_ident), "'")
                    ));
                }
            });
        }
    }

    if matches!(positive_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(positive)]` can only be applied to integer fields"
        )
    {
        field_rules_val.push(quote! {
            if *val <= 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be positive", #field_name_str)
                    ),
                    concat!("Use a positive value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if matches!(negative_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(negative)]` can only be applied to integer fields"
        )
    {
        field_rules_val.push(quote! {
            if *val >= 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be negative", #field_name_str)
                    ),
                    concat!("Use a negative value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if matches!(non_negative_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(non_negative)]` can only be applied to integer fields"
        )
    {
        field_rules_val.push(quote! {
            if *val < 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be non-negative", #field_name_str)
                    ),
                    concat!("Use zero or a positive value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if let Some(min) = min_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(min)]` can only be applied to numeric fields"
        ) {
            if let Some(rhs) = rhs_for_numeric_bound(prim, min, field_ident, &mut compile_errors) {
                field_rules_val.push(quote! {
                    if *val < #rhs {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!("Field '{}' must be at least {}", #field_name_str, #rhs)
                            ),
                            &format!("Ensure '{}' is at least {}", stringify!(#field_ident), #rhs)
                        ));
                    }
                });
            }
        }
    }

    if let Some(max) = max_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(max)]` can only be applied to numeric fields"
        ) {
            if let Some(rhs) = rhs_for_numeric_bound(prim, max, field_ident, &mut compile_errors) {
                field_rules_val.push(quote! {
                    if *val > #rhs {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!("Field '{}' must be at most {}", #field_name_str, #rhs)
                            ),
                            &format!("Ensure '{}' is at most {}", stringify!(#field_ident), #rhs)
                        ));
                    }
                });
            }
        }
    }

    if let Some(start) = starts_with_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(starts_with)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.starts_with(#start) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must start with '{}'", #field_name_str, #start)
                        ),
                        &format!(
                            "Ensure '{}' starts with {}", stringify!(#field_ident), #start
                        )
                    ));
                }
            });
        }
    }

    if let Some(end) = ends_with_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(ends_with)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.ends_with(#end) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must end with '{}'", #field_name_str, #end)
                        ),
                        &format!(
                            "Ensure '{}' ends with {}", stringify!(#field_ident), #end
                        )
                    ));
                }
            });
        }
    }

    if let Some(substr) = includes_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(includes)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.contains(#substr) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must include '{}'", #field_name_str, #substr)
                        ),
                        &format!(
                            "Ensure '{}' includes {}", stringify!(#field_ident), #substr
                        )
                    ));
                }
            });
        }
    }

    if let Some(true) = alphanumeric_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(alphanumeric)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.as_bytes().iter().all(|b| b.is_ascii_alphanumeric()) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must contain only alphanumeric characters", #field_name_str)
                        ),
                        concat!("Ensure '", stringify!(#field_ident), "' has only letters and numbers")
                    ));
                }
            });
        }
    }

    if let Some(multiple) = multiple_of_option {
        if is_type_safe!(
            is_num && is_integer(prim),
            checks,
            field_ident,
            "`#[validate(multiple_of)]` can only be applied to integer fields"
        ) {
            if let Some(rhs) =
                rhs_for_integer_multiple_of(prim, multiple, field_ident, &mut compile_errors)
            {
                field_rules_val.push(quote! {
            if (*val % #rhs) != 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be a multiple of {}", #field_name_str, #rhs)
                    ),
                    &format!("Ensure '{}' is divisible by {}", stringify!(#field_ident), #rhs)
                ));
            }
        });
            }
        }
    }

    checks.extend(compile_errors);

    if !field_rules_direct.is_empty() {
        checks.push(quote! { { #(#field_rules_direct)* } });
    }

    if !field_rules_val.is_empty() {
        let grouped = opt_check!(is_optional, field_ident, {
            #(#field_rules_val)*
        });
        checks.push(grouped);
    }

    checks
}
