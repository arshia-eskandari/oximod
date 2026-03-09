use super::super::{
    args::{BuiltChecks, PrimitiveNum, ValidateArgs},
    helpers::{is_integer, is_signed, rhs_for_integer_multiple_of, rhs_for_numeric_bound},
    macros::is_type_safe,
};
use quote::quote;
use syn::Ident;

pub fn build_numeric_checks(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    is_num: bool,
    prim: PrimitiveNum,
    field_ident: &Ident,
    field_name_lit: &syn::LitStr,
) {
    if let Some(true) = validate_args.positive
        && is_type_safe!(
            is_num && is_signed(prim),
            build_checks.checks,
            field_ident,
            "`#[validate(positive)]` can only be applied to integer fields"
        )
    {
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

    if let Some(true) = validate_args.negative
        && is_type_safe!(
            is_num && is_signed(prim),
            build_checks.checks,
            field_ident,
            "`#[validate(negative)]` can only be applied to integer fields"
        )
    {
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

    if let Some(true) = validate_args.non_negative
        && is_type_safe!(
            is_num && is_signed(prim),
            build_checks.checks,
            field_ident,
            "`#[validate(non_negative)]` can only be applied to integer fields"
        )
    {
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

    if let Some(min) = &validate_args.min
        && is_type_safe!(
            is_num,
            build_checks.checks,
            field_ident,
            "`#[validate(min)]` can only be applied to numeric fields"
        )
        && let Some(rhs) =
            rhs_for_numeric_bound(prim, min, field_ident, &mut build_checks.compile_errors)
    {
        build_checks.min_rhs_ts = Some(rhs);
    }

    if let Some(max) = &validate_args.max
        && is_type_safe!(
            is_num,
            build_checks.checks,
            field_ident,
            "`#[validate(max)]` can only be applied to numeric fields"
        )
        && let Some(rhs) =
            rhs_for_numeric_bound(prim, max, field_ident, &mut build_checks.compile_errors)
    {
        build_checks.max_rhs_ts = Some(rhs);
    }

    if let Some(min_rhs) = &build_checks.min_rhs_ts {
        build_checks.numeric_checks.push(quote! {
            if v < #min_rhs {
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

    if let Some(max_rhs) = &build_checks.max_rhs_ts {
        build_checks.numeric_checks.push(quote! {
            if v > #max_rhs {
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

    if let Some(multiple) = &validate_args.multiple_of
        && is_type_safe!(
            is_num && is_integer(prim),
            build_checks.checks,
            field_ident,
            "`#[validate(multiple_of)]` can only be applied to integer fields"
        )
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
