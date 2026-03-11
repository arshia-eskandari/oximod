use super::super::args::{BuiltChecks, ValidateArgs};
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

pub fn build_string_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
    struct_ident: &Ident,
) {
    if let Some(min_length) = validate_args.min_length {
        build_checks.field_rules_val.push(quote! {
            if val.len() < (#min_length as usize) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must be at least {} characters long",
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
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must be at most {} characters long",
                            #field_name_lit,
                            #max_length
                        )
                    )
                );
            }
        });
    }

    if validate_args.email {
        let re_ident = format_ident!("__oximod_email_re_{}_{}", struct_ident, field_ident);
        let email_pat = syn::LitStr::new(
            r"^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?)+$",
            field_ident.span(),
        );

        build_checks.field_rules_val.push(quote! {
            #[allow(non_upper_case_globals)]
            static #re_ident: ::std::sync::OnceLock<::oximod::_regex::Regex> =
                ::std::sync::OnceLock::new();

            let __re = #re_ident.get_or_init(|| ::oximod::_regex::Regex::new(#email_pat).unwrap());

            if !__re.is_match(val) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must be a valid email address",
                            #field_name_lit
                        )
                    )
                );
            }
        });
    }

    if let Some(pattern) = &validate_args.pattern {
        match ::regex::Regex::new(pattern) {
            Err(e) => {
                let msg = format!(
                    "Invalid regex pattern in validation for '{}': {}",
                    field_ident, e
                );
                build_checks
                    .compile_errors
                    .push(quote_spanned! { field_ident.span() =>
                        compile_error!(#msg);
                    });
            }
            _ => {
                let pattern_lit = syn::LitStr::new(pattern, field_ident.span());

                let re_ident = format_ident!("__oximod_re_{}_{}", struct_ident, field_ident);

                build_checks.field_rules_val.push(quote! {
                    #[allow(non_upper_case_globals)]
                    static #re_ident: ::std::sync::OnceLock<::oximod::_regex::Regex> =
                        ::std::sync::OnceLock::new();

                    let regex = #re_ident
                        .get_or_init(|| ::oximod::_regex::Regex::new(#pattern_lit).unwrap());

                    if !regex.is_match(val) {
                        return Err(
                            ::oximod::_error::oximod_error::OxiModError::validation(
                                format!(
                                    "Field '{}' does not match the required pattern",
                                    #field_name_lit
                                )
                            )
                        );
                    }
                });
            }
        }
    }

    if validate_args.non_empty {
        build_checks.field_rules_val.push(quote! {
            if val.trim().is_empty() {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' must be non-empty", #field_name_lit)
                    )
                );
            }
        });
    }

    if let Some(start) = &validate_args.starts_with {
        build_checks.field_rules_val.push(quote! {
            if !val.starts_with(#start) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must start with '{}'",
                            #field_name_lit,
                            #start
                        )
                    )
                );
            }
        });
    }

    if let Some(end) = &validate_args.ends_with {
        build_checks.field_rules_val.push(quote! {
            if !val.ends_with(#end) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must end with '{}'",
                            #field_name_lit,
                            #end,
                        )
                    )
                );
            }
        });
    }

    if let Some(substr) = &validate_args.includes {
        build_checks.field_rules_val.push(quote! {
            if !val.contains(#substr) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must include '{}'",
                            #field_name_lit,
                            #substr,
                        )
                    )
                );
            }
        });
    }

    if validate_args.alphanumeric {
        build_checks.field_rules_val.push(quote! {
            if !val.as_bytes().iter().all(|b| b.is_ascii_alphanumeric()) {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must contain only alphanumeric characters",
                            #field_name_lit,
                        )
                    )
                );
            }
        });
    }
}
