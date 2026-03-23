use super::super::args::{BuiltChecks, ValidateArgs};
use quote::quote;
use syn::Type;

pub fn build_custom_check(
    build_checks: &mut BuiltChecks,
    validate_args: &ValidateArgs,
    field_ty: &Type,
    opt_inner: Option<&Type>,
) {
    let Some(custom_path) = &validate_args.custom else {
        return;
    };

    let validated_ty = opt_inner.unwrap_or(field_ty);

    build_checks.field_rules_val.push(quote! {
        {
            fn __oximod_check_validator<T, E>(
                f: fn(&T) -> ::std::result::Result<(), E>,
                value: &T,
            ) -> ::std::result::Result<(), E> {
                f(value)
            }

            let __oximod_val: &#validated_ty = val;

            __oximod_check_validator::<#validated_ty, _>(#custom_path, __oximod_val)
                .map_err(|e| ::oximod::OxiModError::validation(e.to_string()))?;
        }
    });
}
