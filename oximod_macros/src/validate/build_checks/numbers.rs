use super::super::{
    args::{BuiltChecks, PrimitiveNum, ValidateArgs},
    helpers::{
        LitNumOperation, compare_lit_num, rhs_for_integer_multiple_of, rhs_for_numeric_bound,
    },
};
use quote::quote;
use syn::Ident;

pub fn build_number_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    prim: PrimitiveNum,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
) {
    if let (Some(min), Some(max)) = (&validate_args.min, &validate_args.max) {
        match compare_lit_num(min, LitNumOperation::Gte, max, field_ident) {
            Ok(true) => {
                build_checks.compile_errors.push(
                    syn::Error::new(field_ident.span(), "`min` must be less than `max`")
                        .to_compile_error(),
                );
            }
            Ok(false) => {}
            Err(err) => {
                build_checks.compile_errors.push(err.to_compile_error());
            }
        }
    }

    let min_rhs_ts = validate_args.min.as_ref().and_then(|min| {
        rhs_for_numeric_bound(prim, min, field_ident, &mut build_checks.compile_errors)
    });

    let max_rhs_ts = validate_args.max.as_ref().and_then(|max| {
        rhs_for_numeric_bound(prim, max, field_ident, &mut build_checks.compile_errors)
    });

    if let Some(min_rhs) = &min_rhs_ts {
        build_checks.numeric_checks.push(quote! {
            if v <= #min_rhs {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must be at least {}",
                            #field_name_lit,
                            #min_rhs,
                        )
                    )
                );
            }
        });
    }

    if let Some(max_rhs) = &max_rhs_ts {
        build_checks.numeric_checks.push(quote! {
            if v >= #max_rhs {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!(
                            "Field '{}' must be at most {}",
                            #field_name_lit,
                            #max_rhs
                        )
                    )
                );
            }
        });
    }
}

pub fn build_signed_number_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_name_lit: &syn::LitStr,
) {
    if validate_args.positive {
        build_checks.field_rules_val.push(quote! {
            if *val <= 0 {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' must be positive", #field_name_lit)
                    )
                );
            }
        });
    }

    if validate_args.negative {
        build_checks.field_rules_val.push(quote! {
            if *val >= 0 {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' must be negative", #field_name_lit)
                    )
                );
            }
        });
    }

    if validate_args.non_negative {
        build_checks.field_rules_val.push(quote! {
            if *val < 0 {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' must be non-negative", #field_name_lit)
                    )
                );
            }
        });
    }

    if validate_args.non_positive {
        build_checks.field_rules_val.push(quote! {
            if *val > 0 {
                return Err(
                    ::oximod::_error::oximod_error::OxiModError::validation(
                        format!("Field '{}' must be non-positive", #field_name_lit)
                    )
                );
            }
        });
    }
}

pub fn build_integer_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    prim: PrimitiveNum,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
) {
    if let Some(multiple) = &validate_args.multiple_of
        && let Some((rhs, pow2_mask)) = rhs_for_integer_multiple_of(
            prim,
            multiple,
            field_ident,
            &mut build_checks.compile_errors,
        )
    {
        if let Some(mask_lit) = pow2_mask {
            build_checks.numeric_checks.push(quote! {
                if (v & #mask_lit) != 0 {
                    return Err(
                        ::oximod::_error::oximod_error::OxiModError::validation(
                            format!(
                                "Field '{}' must be a multiple of {}",
                                #field_name_lit,
                                #rhs,
                            )
                        )
                    );
                }
            });
        } else {
            build_checks.numeric_checks.push(quote! {
                if (v % #rhs) != 0 {
                    return Err(
                        ::oximod::_error::oximod_error::OxiModError::validation(
                            format!(
                                "Field '{}' must be a multiple of {}",
                                #field_name_lit,
                                #rhs,
                            )
                        )
                    );
                }
            });
        }
    }
}
