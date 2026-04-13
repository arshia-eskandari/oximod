use super::super::args::{BuiltChecks, ValidateArgs};
use quote::quote;
use syn::{LitStr, Type};

pub fn build_custom_check(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ty: &Type,
    opt_inner: Option<&Type>,
    field_name_lit: &LitStr,
) {
    let Some(custom_path) = &validate_args.custom else {
        return;
    };

    let validated_ty = opt_inner.unwrap_or(field_ty);

    build_checks.field_rules_val.push(quote! {
        {
            let __oximod_val: &#validated_ty = val;

            if let Err(e) = #custom_path(__oximod_val) {
                validation_errors.push(::oximod::ValidationError::new(
                    #field_name_lit,
                    e.to_string(),
                ));
            }
        }
    });
}
