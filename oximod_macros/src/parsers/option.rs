//! Parsing helpers for simple `Option<T>` field types.
//!
//! These helpers intentionally recognize only an unqualified `Option<T>`
//! path. Other path types, references, and malformed generic arguments are
//! left unchanged.

use syn::{GenericArgument, PathArguments, Type};

/// Returns the inner type when `ty` is a simple `Option<T>`.
///
/// Qualified paths such as `std::option::Option<T>` are not treated as
/// `Option<T>` because derive input normally uses the standard prelude form.
pub fn option_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
        return None;
    }

    let segment = type_path.path.segments.first()?;

    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    if arguments.args.len() != 1 {
        return None;
    }

    match arguments.args.first()? {
        GenericArgument::Type(inner_type) => Some(inner_type),
        _ => None,
    }
}

/// Returns the inner type when `ty` is a simple `Option<T>`.
///
/// This is retained as a second entry point for existing parser callers and
/// delegates to [`option_inner_type`] so both helpers behave identically.
#[inline]
pub fn unwrap_option_type(ty: &Type) -> Option<&Type> {
    option_inner_type(ty)
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{Type, parse_quote};

    use super::{option_inner_type, unwrap_option_type};

    #[test]
    fn returns_inner_type_for_option() {
        let ty: Type = parse_quote! {
            Option<String>
        };

        let inner = option_inner_type(&ty).expect("Option should have an inner type");

        assert_eq!(compact(inner), "String");
    }

    #[test]
    fn preserves_nested_inner_types() {
        let ty: Type = parse_quote! {
            Option<Vec<String>>
        };

        let inner = option_inner_type(&ty).expect("Option should have an inner type");

        assert_eq!(compact(inner), "Vec<String>");
    }

    #[test]
    fn non_option_types_return_none() {
        let ty: Type = parse_quote! {
            String
        };

        assert!(option_inner_type(&ty).is_none());
    }

    #[test]
    fn references_to_options_return_none() {
        let ty: Type = parse_quote! {
            &Option<String>
        };

        assert!(option_inner_type(&ty).is_none());
    }

    #[test]
    fn qualified_option_paths_return_none() {
        let ty: Type = parse_quote! {
            std::option::Option<String>
        };

        assert!(option_inner_type(&ty).is_none());
    }

    #[test]
    fn non_type_generic_arguments_return_none() {
        let ty: Type = parse_quote! {
            Option<'static>
        };

        assert!(option_inner_type(&ty).is_none());
    }

    #[test]
    fn both_entry_points_have_identical_behavior() {
        let option: Type = parse_quote! {
            Option<u32>
        };
        let non_option: Type = parse_quote! {
            u32
        };

        assert_eq!(
            unwrap_option_type(&option).map(compact),
            option_inner_type(&option).map(compact),
        );

        assert_eq!(
            unwrap_option_type(&non_option).map(compact),
            option_inner_type(&non_option).map(compact),
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
