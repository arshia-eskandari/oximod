//! Optional-field validation token generation.
//!
//! The `required` rule applies to `Option<T>` fields and produces a validation
//! error when the field contains `None`.
//!
//! Unlike rules that validate an inner `val`, this check must inspect the
//! model field directly, so it is stored in `field_rules_direct`.

use quote::quote;
use syn::{Ident, LitStr};

use super::super::args::{BuiltChecks, ValidateArgs};

/// Appends validation checks that operate directly on an optional field.
pub fn build_option_checks(
    built_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ident: &Ident,
    field_name_lit: &LitStr,
) {
    if !validate_args.required {
        return;
    }

    built_checks.field_rules_direct.push(quote! {
        if self.#field_ident.is_none() {
            validation_errors.push(
                ::oximod::ValidationError::new(
                    #field_name_lit,
                    "is required",
                ),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::{ToTokens, format_ident, quote};
    use syn::LitStr;

    use crate::validate::args::{BuiltChecks, ValidateArgs};

    use super::build_option_checks;

    #[test]
    fn required_rule_generates_direct_none_check() {
        let mut built_checks = BuiltChecks::default();

        let validate_args = ValidateArgs {
            required: true,
            ..Default::default()
        };

        let field_ident = format_ident!("nickname");

        let field_name = LitStr::new("nickname", Span::call_site());

        build_option_checks(&mut built_checks, &validate_args, &field_ident, &field_name);

        assert_eq!(built_checks.field_rules_direct.len(), 1);

        let generated = compact(&built_checks.field_rules_direct[0]);

        assert!(
            generated.contains("ifself.nickname.is_none()"),
            "required validation should inspect the field directly: \
             {generated}"
        );

        assert!(
            generated.contains(
                "::oximod::ValidationError::new(\
\"nickname\",\"isrequired\",)"
            ),
            "required validation should retain the field name and message: \
             {generated}"
        );
    }

    #[test]
    fn absent_required_rule_generates_no_check() {
        let mut built_checks = BuiltChecks::default();
        let validate_args = ValidateArgs::default();
        let field_ident = format_ident!("nickname");

        let field_name = LitStr::new("nickname", Span::call_site());

        build_option_checks(&mut built_checks, &validate_args, &field_ident, &field_name);

        assert!(
            built_checks.field_rules_direct.is_empty(),
            "fields without `required` should generate no direct check"
        );
    }

    #[test]
    fn required_check_is_appended_after_existing_rules() {
        let existing = quote! {
            existing_check();
        };

        let mut built_checks = BuiltChecks {
            field_rules_direct: vec![existing.clone()],
            ..Default::default()
        };

        let validate_args = ValidateArgs {
            required: true,
            ..Default::default()
        };

        let field_ident = format_ident!("email");

        let field_name = LitStr::new("email", Span::call_site());

        build_option_checks(&mut built_checks, &validate_args, &field_ident, &field_name);

        assert_eq!(built_checks.field_rules_direct.len(), 2);

        assert_eq!(
            compact(&built_checks.field_rules_direct[0]),
            compact(&existing),
            "existing direct checks should remain unchanged"
        );

        assert!(
            compact(&built_checks.field_rules_direct[1]).contains("self.email.is_none()"),
            "the required check should be appended"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
