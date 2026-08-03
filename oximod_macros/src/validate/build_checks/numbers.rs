//! Numeric-validation token generation.
//!
//! Numeric validation is divided into three groups:
//!
//! - minimum and maximum bounds for all numeric primitives;
//! - sign checks for signed integers and floating-point values;
//! - `multiple_of` checks for integer values.
//!
//! Numeric literals are validated and converted by the helpers in
//! `validate::helpers::numeric_bounds`.

use quote::quote;
use syn::{Ident, LitStr};

use super::super::{
    args::{BuiltChecks, PrimitiveNum, ValidateArgs},
    helpers::{
        LitNumOperation, compare_lit_num, rhs_for_integer_multiple_of, rhs_for_numeric_bound,
    },
};

/// Appends minimum and maximum validation checks.
///
/// Invalid ranges and bounds that cannot be represented by the field's
/// primitive type are added to `compile_errors`.
pub fn build_number_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    primitive: PrimitiveNum,
    field_ident: &Ident,
    field_name_lit: &LitStr,
) {
    if let (Some(minimum), Some(maximum)) = (&validate_args.min, &validate_args.max) {
        match compare_lit_num(minimum, LitNumOperation::Gte, maximum, field_ident) {
            Ok(true) => {
                built_checks.compile_errors.push(
                    syn::Error::new(field_ident.span(), "`min` must be less than `max`")
                        .to_compile_error(),
                );
            }

            Ok(false) => {}

            Err(error) => {
                built_checks.compile_errors.push(error.to_compile_error());
            }
        }
    }

    let minimum_rhs = validate_args.min.as_ref().and_then(|minimum| {
        rhs_for_numeric_bound(
            primitive,
            minimum,
            field_ident,
            &mut built_checks.compile_errors,
        )
    });

    let maximum_rhs = validate_args.max.as_ref().and_then(|maximum| {
        rhs_for_numeric_bound(
            primitive,
            maximum,
            field_ident,
            &mut built_checks.compile_errors,
        )
    });

    if let Some(minimum_rhs) = &minimum_rhs {
        let (operator, message) = if validate_args.min_exclusive {
            (quote! { <= }, "more than")
        } else {
            (quote! { < }, "at least")
        };

        built_checks.numeric_checks.push(quote! {
            if v #operator #minimum_rhs {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must be {} {}",
                            #message,
                            #minimum_rhs,
                        ),
                    ),
                );
            }
        });
    }

    if let Some(maximum_rhs) = &maximum_rhs {
        let (operator, message) = if validate_args.max_exclusive {
            (quote! { >= }, "less than")
        } else {
            (quote! { > }, "at most")
        };

        built_checks.numeric_checks.push(quote! {
            if v #operator #maximum_rhs {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must be {} {}",
                            #message,
                            #maximum_rhs,
                        ),
                    ),
                );
            }
        });
    }
}

/// Appends sign-based validation checks.
///
/// Integer fields compare against `0`; floating-point fields compare against
/// `0.0`.
pub fn build_signed_number_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_name_lit: &LitStr,
    is_integer: bool,
) {
    let zero = if is_integer {
        quote! { 0 }
    } else {
        quote! { 0.0 }
    };

    if validate_args.positive {
        built_checks.field_rules_val.push(quote! {
            if *val <= #zero {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be positive",
                    ),
                );
            }
        });
    }

    if validate_args.negative {
        built_checks.field_rules_val.push(quote! {
            if *val >= #zero {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be negative",
                    ),
                );
            }
        });
    }

    if validate_args.non_negative {
        built_checks.field_rules_val.push(quote! {
            if *val < #zero {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be non-negative",
                    ),
                );
            }
        });
    }

    if validate_args.non_positive {
        built_checks.field_rules_val.push(quote! {
            if *val > #zero {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        "must be non-positive",
                    ),
                );
            }
        });
    }
}

/// Appends an integer `multiple_of` check.
///
/// Unsigned power-of-two divisors use a bitmask. Other divisors use the
/// remainder operator.
pub fn build_integer_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    primitive: PrimitiveNum,
    field_ident: &Ident,
    field_name_lit: &LitStr,
) {
    let Some(multiple) = &validate_args.multiple_of else {
        return;
    };

    let Some((rhs, power_of_two_mask)) = rhs_for_integer_multiple_of(
        primitive,
        multiple,
        field_ident,
        &mut built_checks.compile_errors,
    ) else {
        return;
    };

    if let Some(mask_literal) = power_of_two_mask {
        built_checks.numeric_checks.push(quote! {
            if (v & #mask_literal) != 0 {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must be a multiple of {}",
                            #rhs,
                        ),
                    ),
                );
            }
        });
    } else {
        built_checks.numeric_checks.push(quote! {
            if (v % #rhs) != 0 {
                validation_errors.push(
                    ::oximod::ValidationError::new(
                        #field_name_lit,
                        format!(
                            "must be a multiple of {}",
                            #rhs,
                        ),
                    ),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::{ToTokens, format_ident};
    use syn::{LitStr, parse_quote};

    use crate::validate::args::{BuiltChecks, LitNum, PrimitiveNum, ValidateArgs};

    use super::{build_integer_checks, build_number_checks, build_signed_number_checks};

    #[test]
    fn absent_numeric_rules_generate_no_checks() {
        let mut built_checks = BuiltChecks::default();
        let validate_args = ValidateArgs::default();
        let field_ident = format_ident!("age");

        let field_name = LitStr::new("age", Span::call_site());

        build_number_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        build_signed_number_checks(&mut built_checks, &validate_args, &field_name, true);

        build_integer_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        assert!(built_checks.numeric_checks.is_empty());
        assert!(built_checks.field_rules_val.is_empty());
        assert!(built_checks.compile_errors.is_empty());
    }

    #[test]
    fn inclusive_numeric_bounds_generate_expected_comparisons() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(18),
                neg: false,
            }),
            max: Some(LitNum::Int {
                lit: parse_quote!(65),
                neg: false,
            }),
            ..Default::default()
        };

        let field_ident = format_ident!("age");

        let field_name = LitStr::new("age", Span::call_site());

        build_number_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        assert_eq!(built_checks.numeric_checks.len(), 2);
        assert!(built_checks.compile_errors.is_empty());

        let minimum = compact(&built_checks.numeric_checks[0]);

        let maximum = compact(&built_checks.numeric_checks[1]);

        assert!(
            minimum.contains("ifv<18"),
            "inclusive minimum should reject values below the bound: \
             {minimum}"
        );

        assert!(
            minimum.contains("format!(\"mustbe{}{}\",\"atleast\",18,)"),
            "minimum should use the inclusive message: {minimum}"
        );

        assert!(
            maximum.contains("ifv>65"),
            "inclusive maximum should reject values above the bound: \
             {maximum}"
        );

        assert!(
            maximum.contains("format!(\"mustbe{}{}\",\"atmost\",65,)"),
            "maximum should use the inclusive message: {maximum}"
        );
    }

    #[test]
    fn exclusive_numeric_bounds_generate_expected_comparisons() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(18),
                neg: false,
            }),
            max: Some(LitNum::Int {
                lit: parse_quote!(65),
                neg: false,
            }),
            min_exclusive: true,
            max_exclusive: true,
            ..Default::default()
        };

        let field_ident = format_ident!("age");

        let field_name = LitStr::new("age", Span::call_site());

        build_number_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        let minimum = compact(&built_checks.numeric_checks[0]);

        let maximum = compact(&built_checks.numeric_checks[1]);

        assert!(
            minimum.contains("ifv<=18"),
            "exclusive minimum should reject the bound itself: \
             {minimum}"
        );

        assert!(
            minimum.contains("format!(\"mustbe{}{}\",\"morethan\",18,)"),
            "minimum should use the exclusive message: {minimum}"
        );

        assert!(
            maximum.contains("ifv>=65"),
            "exclusive maximum should reject the bound itself: \
             {maximum}"
        );

        assert!(
            maximum.contains("format!(\"mustbe{}{}\",\"lessthan\",65,)"),
            "maximum should use the exclusive message: {maximum}"
        );
    }

    #[test]
    fn invalid_numeric_range_generates_compile_error() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(10),
                neg: false,
            }),
            max: Some(LitNum::Int {
                lit: parse_quote!(5),
                neg: false,
            }),
            ..Default::default()
        };

        let field_ident = format_ident!("value");

        let field_name = LitStr::new("value", Span::call_site());

        build_number_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        assert_eq!(built_checks.compile_errors.len(), 1);

        let error = compact(&built_checks.compile_errors[0]);

        assert!(
            error.contains("compile_error!"),
            "invalid range should generate compile-error tokens: \
             {error}"
        );

        assert!(
            error.contains("`min`mustbelessthan`max`"),
            "diagnostic should explain the invalid range: {error}"
        );
    }

    #[test]
    fn integer_sign_rules_use_integer_zero() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            positive: true,
            negative: true,
            non_negative: true,
            non_positive: true,
            ..Default::default()
        };

        let field_name = LitStr::new("value", Span::call_site());

        build_signed_number_checks(&mut built_checks, &validate_args, &field_name, true);

        assert_eq!(built_checks.field_rules_val.len(), 4);

        assert!(compact(&built_checks.field_rules_val[0]).contains("if*val<=0"));

        assert!(compact(&built_checks.field_rules_val[1]).contains("if*val>=0"));

        assert!(compact(&built_checks.field_rules_val[2]).contains("if*val<0"));

        assert!(compact(&built_checks.field_rules_val[3]).contains("if*val>0"));
    }

    #[test]
    fn float_sign_rules_use_float_zero() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            positive: true,
            ..Default::default()
        };

        let field_name = LitStr::new("score", Span::call_site());

        build_signed_number_checks(&mut built_checks, &validate_args, &field_name, false);

        assert_eq!(built_checks.field_rules_val.len(), 1);

        let generated = compact(&built_checks.field_rules_val[0]);

        assert!(
            generated.contains("if*val<=0.0"),
            "float sign checks should compare against 0.0: \
             {generated}"
        );

        assert!(
            generated.contains("\"mustbepositive\""),
            "positive check should generate its message: \
             {generated}"
        );
    }

    #[test]
    fn unsigned_power_of_two_multiple_uses_bitmask() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            multiple_of: Some(parse_quote!(8)),
            ..Default::default()
        };

        let field_ident = format_ident!("count");

        let field_name = LitStr::new("count", Span::call_site());

        build_integer_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::U32,
            &field_ident,
            &field_name,
        );

        assert_eq!(built_checks.numeric_checks.len(), 1);
        assert!(built_checks.compile_errors.is_empty());

        let generated = compact(&built_checks.numeric_checks[0]);

        assert!(
            generated.contains("if(v&7)!=0"),
            "unsigned power-of-two divisor should use a mask: \
             {generated}"
        );

        assert!(
            generated.contains("\"mustbeamultipleof{}\""),
            "multiple_of should generate its validation message: \
             {generated}"
        );
    }

    #[test]
    fn signed_multiple_uses_remainder_operator() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            multiple_of: Some(parse_quote!(8)),
            ..Default::default()
        };

        let field_ident = format_ident!("count");

        let field_name = LitStr::new("count", Span::call_site());

        build_integer_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        assert_eq!(built_checks.numeric_checks.len(), 1);
        assert!(built_checks.compile_errors.is_empty());

        let generated = compact(&built_checks.numeric_checks[0]);

        assert!(
            generated.contains("if(v%8)!=0"),
            "signed integer divisors should use remainder: \
             {generated}"
        );

        assert!(
            !generated.contains("v&"),
            "signed integers should not use a bitmask: \
             {generated}"
        );
    }

    #[test]
    fn numeric_checks_append_after_existing_rules() {
        let existing = quote::quote! {
            existing_check();
        };

        let mut built_checks = BuiltChecks {
            numeric_checks: vec![existing.clone()],
            ..Default::default()
        };

        let validate_args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(1),
                neg: false,
            }),
            ..Default::default()
        };

        let field_ident = format_ident!("value");

        let field_name = LitStr::new("value", Span::call_site());

        build_number_checks(
            &mut built_checks,
            &validate_args,
            PrimitiveNum::I32,
            &field_ident,
            &field_name,
        );

        assert_eq!(built_checks.numeric_checks.len(), 2);

        assert_eq!(
            compact(&built_checks.numeric_checks[0]),
            compact(&existing),
            "existing numeric checks should remain unchanged"
        );

        assert!(
            compact(&built_checks.numeric_checks[1]).contains("ifv<1"),
            "new numeric check should be appended"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
