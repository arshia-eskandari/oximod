use syn::Type;

macro_rules! path_type_check {
    ($fn_name:ident, $target:ident) => {
        #[inline]
        pub fn $fn_name(ty: &syn::Type) -> bool {
            match ty {
                syn::Type::Path(tp) => tp
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == stringify!($target)),
                _ => false,
            }
        }
    };
}

path_type_check!(is_vec, Vec);
path_type_check!(is_vecdeque, VecDeque);
path_type_check!(is_hashset, HashSet);
path_type_check!(is_btreeset, BTreeSet);
path_type_check!(is_hashmap, HashMap);
path_type_check!(is_btreemap, BTreeMap);

#[inline]
pub fn is_array(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Array(_))
}

#[inline]
pub fn is_string(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            tp.path.is_ident("String")
                || tp.path.segments.last().is_some_and(|seg| {
                    seg.ident == "Cow"
                        && matches!(&seg.arguments, syn::PathArguments::AngleBracketed(ab)
                        if ab.args.iter().any(|arg| matches!(
                            arg,
                            syn::GenericArgument::Type(syn::Type::Path(p))
                                if p.path.is_ident("str")
                        )))
                })
        }
        Type::Reference(r) => match &*r.elem {
            Type::Path(tp) => tp.path.is_ident("str") || tp.path.is_ident("String"),
            _ => false,
        },
        _ => false,
    }
}

