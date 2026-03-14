use syn::{GenericArgument, PathArguments, Type};

/// If the provided type is `Option<Inner>`, returns a reference to the inner type; otherwise returns `None`.
pub fn unwrap_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(generic_args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = generic_args.args.first()
    {
        return Some(inner_ty);
    }
    None
}

/// If `ty` is `Option<Inner>`, returns `Some(&Inner)`, otherwise `None`.
pub fn option_inner_type(ty: &Type) -> Option<&Type> {
    // We only care about a simple `Option<...>` path type
    if let Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
    {
        let segment = &type_path.path.segments[0];
        if segment.ident == "Option"
            && let PathArguments::AngleBracketed(params) = &segment.arguments
            && params.args.len() == 1
            && let GenericArgument::Type(inner_ty) = &params.args[0]
        {
            return Some(inner_ty);
        }
    }
    None
}
