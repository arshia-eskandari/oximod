//! Field-validation token coordination.
//!
//! This module connects parsed `ValidateArgs` with the specialized validation
//! builders. For each field, it:
//!
//! - rejects rules that require incompatible field-type groups;
//! - verifies numeric, string, length, integer, signed, and optional types;
//! - generates compile-time diagnostics for unsupported combinations;
//! - collects runtime validation rules;
//! - groups optional-field rules behind `Some(value)` checks;
//! - preserves direct checks such as `required`.

use super::args::{BuiltChecks, ValidateArgs};
use super::build_checks::{
    custom_checks::build_custom_check,
    length_types::build_length_checks,
    numbers::{build_integer_checks, build_number_checks, build_signed_number_checks},
    options::build_option_checks,
    strings::build_string_checks,
};
use super::helpers::{is_integer, is_length_type, is_numeric, is_signed, is_string, primitive_of};
use super::macros::opt_check;
use crate::parsers::unwrap_option_type;
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
    if validate_args.has_type_collision() {
        return vec![quote_spanned! { field_ident.span() =>
            compile_error!(
                "invalid validation rules: cannot apply validations from different type groups to the same field"
            );
        }];
    }

    let mut build_checks = BuiltChecks::default();
    let field_name_lit = syn::LitStr::new(&field_ident.to_string(), field_ident.span());
    let opt_inner = unwrap_option_type(field_ty);
    let inner_ty = opt_inner.unwrap_or(field_ty);
    let prim = primitive_of(inner_ty);
    let is_str = is_string(inner_ty);

    if validate_args.must_be_number() && !is_numeric(&prim) {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses numeric validation rules, but its type is not numeric"
                    )
                );
            });
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
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses signed-number validation rules, but its type is not a signed numeric type"
                    )
                );
            });
    } else {
        build_signed_number_checks(
            &mut build_checks,
            &validate_args,
            &field_name_lit,
            is_integer(&prim),
        );
    }

    if validate_args.must_be_integer() && !is_integer(&prim) {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses integer-only validation rules, but its type is not an integer"
                    )
                );
            });
    } else {
        build_integer_checks(
            &mut build_checks,
            &validate_args,
            prim,
            field_ident,
            &field_name_lit,
        );
    }

    if validate_args.must_be_string() && !is_str {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses string validation rules, but its type is not String or &str"
                    )
                );
            });
    } else {
        build_string_checks(
            &mut build_checks,
            &validate_args,
            field_ident,
            &field_name_lit,
            struct_ident,
        );
    }

    if validate_args.must_be_length_type() && !is_length_type(inner_ty) {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses length validation rules, but its type does not support length"
                    )
                );
            });
    } else {
        build_length_checks(&mut build_checks, &validate_args, &field_name_lit, is_str);
    }

    if validate_args.must_be_optional() && opt_inner.is_none() {
        build_checks
            .compile_errors
            .push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' uses option validation rules, but its type is not Option<T>"
                    )
                );
            });
    } else {
        build_option_checks(
            &mut build_checks,
            &validate_args,
            field_ident,
            &field_name_lit,
        );
    }

    build_custom_check(
        &mut build_checks,
        &validate_args,
        field_ty,
        opt_inner,
        &field_name_lit,
    );

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

    // Nested descent runs after the field's own rules, so container-level
    // errors (`required`, `non_empty`, ...) precede descendant errors. The
    // whole field value is handed to `NestedValidate`; container adapters
    // handle `Option`/`Vec`/`HashMap` unwrapping, and an unsupported field
    // type fails to compile at this call with the field's span.
    if validate_args.nested {
        build_checks
            .checks
            .push(quote_spanned! { field_ident.span() =>
                ::oximod::_feature::nested::NestedValidate::collect_nested_validation_errors(
                    &self.#field_ident,
                    #field_name_lit,
                    &mut validation_errors,
                );
            });
    }

    build_checks.checks
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::format_ident;
    use syn::{Type, parse_quote};

    use crate::validate::args::{LitNum, ValidateArgs};

    use super::generate_validate_model_tokens;

    #[test]
    fn empty_validation_arguments_generate_no_tokens() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("name");
        let field_ty: Type = parse_quote!(String);

        let generated = generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            ValidateArgs::default(),
        );

        assert!(
            generated.is_empty(),
            "fields without validation rules should generate no checks"
        );
    }

    #[test]
    fn incompatible_validation_groups_generate_compile_error() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("value");
        let field_ty: Type = parse_quote!(String);

        let args = ValidateArgs {
            email: true,
            min: Some(LitNum::Int {
                lit: parse_quote!(1),
                neg: false,
            }),
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("compile_error!"),
            "type collisions should generate a compile error: \
             {generated}"
        );

        assert!(
            generated.contains("cannotapplyvalidationsfromdifferenttypegroups"),
            "compile error should explain the type collision: \
             {generated}"
        );
    }

    #[test]
    fn numeric_rules_reject_non_numeric_fields() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("age");
        let field_ty: Type = parse_quote!(String);

        let args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(18),
                neg: false,
            }),
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("compile_error!"),
            "numeric validation on String should fail: \
             {generated}"
        );

        assert!(
            generated.contains("usesnumericvalidationrules,butitstypeisnotnumeric"),
            "diagnostic should identify numeric type mismatch: \
             {generated}"
        );
    }

    #[test]
    fn string_rules_reject_non_string_fields() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("email");
        let field_ty: Type = parse_quote!(i32);

        let args = ValidateArgs {
            email: true,
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("compile_error!"),
            "string validation on i32 should fail: \
             {generated}"
        );

        assert!(
            generated.contains("usesstringvalidationrules,butitstypeisnotStringor&str"),
            "diagnostic should identify string type mismatch: \
             {generated}"
        );
    }

    #[test]
    fn required_rule_rejects_non_optional_fields() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("nickname");
        let field_ty: Type = parse_quote!(String);

        let args = ValidateArgs {
            required: true,
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("compile_error!"),
            "required validation should require Option<T>: \
             {generated}"
        );

        assert!(
            generated.contains("usesoptionvalidationrules,butitstypeisnotOption<T>"),
            "diagnostic should identify optional type mismatch: \
             {generated}"
        );
    }

    #[test]
    fn required_rule_generates_direct_optional_check() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("nickname");
        let field_ty: Type = parse_quote! {
            Option<String>
        };

        let args = ValidateArgs {
            required: true,
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("ifself.nickname.is_none()"),
            "required validation should inspect the Option directly: \
             {generated}"
        );

        assert!(
            generated.contains("\"isrequired\""),
            "required validation should generate its error message: \
             {generated}"
        );

        assert!(
            !generated.contains("compile_error!"),
            "valid required rule should not generate compile errors: \
             {generated}"
        );
    }

    #[test]
    fn optional_field_rules_run_only_when_value_is_present() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("nickname");
        let field_ty: Type = parse_quote! {
            Option<String>
        };

        let args = ValidateArgs {
            min_length: Some(3),
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("ifletSome(val)=&self.nickname"),
            "optional field rules should run only for Some: \
             {generated}"
        );

        assert!(
            generated.contains("ifval.len()<"),
            "minimum-length check should be generated: \
             {generated}"
        );

        assert!(
            !generated.contains("compile_error!"),
            "valid optional length rule should compile: \
             {generated}"
        );
    }

    #[test]
    fn valid_numeric_bounds_generate_runtime_checks() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("age");
        let field_ty: Type = parse_quote!(i32);

        let args = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(18),
                neg: false,
            }),
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains("letv=*val;"),
            "numeric checks should copy the primitive value: \
             {generated}"
        );

        assert!(
            generated.contains("ifv<18"),
            "minimum bound should generate a comparison: \
             {generated}"
        );

        assert!(
            generated.contains("\"age\""),
            "generated validation error should retain the field name: \
             {generated}"
        );

        assert!(
            !generated.contains("compile_error!"),
            "valid numeric rules should not generate compile errors: \
             {generated}"
        );
    }

    #[test]
    fn nested_rule_generates_a_collect_call_on_the_whole_field() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("address");
        let field_ty: Type = parse_quote!(Address);

        let args = ValidateArgs {
            nested: true,
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        assert!(
            generated.contains(
                "::oximod::_feature::nested::NestedValidate::collect_nested_validation_errors(\
&self.address,\"address\",&mutvalidation_errors,);"
            ),
            "nested rule should collect through the hidden trait: {generated}"
        );

        assert!(
            !generated.contains("compile_error!"),
            "nested alone should not generate compile errors: {generated}"
        );
    }

    #[test]
    fn nested_descent_runs_after_the_fields_own_rules() {
        let struct_ident = format_ident!("User");
        let field_ident = format_ident!("address");
        let field_ty: Type = parse_quote!(Option<Address>);

        let args = ValidateArgs {
            required: true,
            nested: true,
            ..Default::default()
        };

        let generated = compact(&generate_validate_model_tokens(
            &struct_ident,
            &field_ident,
            &field_ty,
            args,
        ));

        let required = generated
            .find("ifself.address.is_none()")
            .expect("required check should be generated");

        let nested = generated
            .find("collect_nested_validation_errors")
            .expect("nested collect should be generated");

        assert!(
            required < nested,
            "container-level rules should precede nested descent: {generated}"
        );

        assert!(
            generated.contains("&self.address,"),
            "nested descent should receive the whole Option field so the \
             container adapter handles None: {generated}"
        );
    }

    fn compact(tokens: &[TokenStream]) -> String {
        tokens
            .iter()
            .map(TokenStream::to_string)
            .collect::<String>()
            .replace(' ', "")
    }
}
