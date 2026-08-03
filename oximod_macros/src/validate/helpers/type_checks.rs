//! Field-type predicates used by validation generation.
//!
//! These helpers recognize common string and collection types from their
//! final path segment, allowing both imported and qualified forms such as:
//!
//! - `Vec<T>` and `std::vec::Vec<T>`;
//! - `String` and `std::string::String`;
//! - `Cow<'a, str>` and `std::borrow::Cow<'a, str>`.
//!
//! They intentionally inspect syntax only. Trait implementations and aliases
//! cannot be resolved during procedural-macro expansion.

use syn::{GenericArgument, Path, PathArguments, Type};

macro_rules! path_type_check {
    ($fn_name:ident, $target:ident) => {
        #[inline]
        pub fn $fn_name(ty: &Type) -> bool {
            let Type::Path(type_path) = ty else {
                return false;
            };

            path_ends_with(&type_path.path, stringify!($target))
        }
    };
}

path_type_check!(is_vec, Vec);
path_type_check!(is_vecdeque, VecDeque);
path_type_check!(is_hashset, HashSet);
path_type_check!(is_btreeset, BTreeSet);
path_type_check!(is_hashmap, HashMap);
path_type_check!(is_btreemap, BTreeMap);

/// Returns whether the type is a fixed-size array.
#[inline]
pub fn is_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(_))
}

/// Returns whether the type is string-like.
///
/// Supported forms are:
///
/// - `String`;
/// - `&str`;
/// - `&String`;
/// - `Cow<'a, str>`;
/// - qualified versions of these path types.
#[inline]
pub fn is_string(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            path_ends_with(&type_path.path, "String") || is_cow_str(&type_path.path)
        }

        Type::Reference(reference) => {
            let Type::Path(type_path) = reference.elem.as_ref() else {
                return false;
            };

            path_ends_with(&type_path.path, "str") || path_ends_with(&type_path.path, "String")
        }

        _ => false,
    }
}

fn is_cow_str(path: &Path) -> bool {
    let Some(segment) = path.segments.last() else {
        return false;
    };

    if segment.ident != "Cow" {
        return false;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };

    arguments.args.iter().any(|argument| {
        matches!(
            argument,
            GenericArgument::Type(Type::Path(type_path))
                if path_ends_with(&type_path.path, "str")
        )
    })
}

fn path_ends_with(path: &Path, target: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == target)
}

#[cfg(test)]
mod tests {
    use syn::{Type, parse_quote, parse_str};

    use super::{
        is_array, is_btreemap, is_btreeset, is_hashmap, is_hashset, is_string, is_vec, is_vecdeque,
    };

    type TypePredicate = fn(&Type) -> bool;

    #[test]
    fn recognizes_supported_collection_types() {
        let cases: [(&str, TypePredicate); 6] = [
            ("Vec<u8>", is_vec),
            ("VecDeque<u8>", is_vecdeque),
            ("HashSet<u8>", is_hashset),
            ("BTreeSet<u8>", is_btreeset),
            ("HashMap<String, u8>", is_hashmap),
            ("BTreeMap<String, u8>", is_btreemap),
        ];

        for (source, predicate) in cases {
            let ty = parse_str::<Type>(source).expect("collection type should parse");

            assert!(predicate(&ty), "{source} should be recognized");
        }
    }

    #[test]
    fn recognizes_qualified_collection_paths() {
        let cases: [(&str, TypePredicate); 6] = [
            ("std::vec::Vec<u8>", is_vec),
            ("std::collections::VecDeque<u8>", is_vecdeque),
            ("std::collections::HashSet<u8>", is_hashset),
            ("std::collections::BTreeSet<u8>", is_btreeset),
            ("std::collections::HashMap<String, u8>", is_hashmap),
            ("std::collections::BTreeMap<String, u8>", is_btreemap),
        ];

        for (source, predicate) in cases {
            let ty = parse_str::<Type>(source).expect("qualified type should parse");

            assert!(predicate(&ty), "{source} should be recognized");
        }
    }

    #[test]
    fn collection_predicates_reject_other_types() {
        let ty: Type = parse_quote! {
            CustomVec<u8>
        };

        assert!(!is_vec(&ty));
        assert!(!is_vecdeque(&ty));
        assert!(!is_hashset(&ty));
        assert!(!is_btreeset(&ty));
        assert!(!is_hashmap(&ty));
        assert!(!is_btreemap(&ty));
    }

    #[test]
    fn recognizes_fixed_size_arrays() {
        let array: Type = parse_quote! {
            [u8; 4]
        };

        let vector: Type = parse_quote! {
            Vec<u8>
        };

        assert!(is_array(&array));
        assert!(!is_array(&vector));
    }

    #[test]
    fn recognizes_owned_and_borrowed_strings() {
        let types: [Type; 5] = [
            parse_quote!(String),
            parse_quote!(std::string::String),
            parse_quote!(&str),
            parse_quote!(&String),
            parse_quote!(&std::string::String),
        ];

        for ty in types {
            assert!(is_string(&ty), "string type should be recognized");
        }
    }

    #[test]
    fn recognizes_owned_and_qualified_cow_strings() {
        let types: [Type; 2] = [
            parse_quote!(Cow<'static, str>),
            parse_quote!(std::borrow::Cow<'static, str>),
        ];

        for ty in types {
            assert!(is_string(&ty), "Cow<str> should be recognized");
        }
    }

    #[test]
    fn rejects_non_string_types() {
        let types: [Type; 6] = [
            parse_quote!(i32),
            parse_quote!(Vec<String>),
            parse_quote!(Box<str>),
            parse_quote!(Cow<'static, String>),
            parse_quote!([u8; 4]),
            parse_quote!((String, String)),
        ];

        for ty in types {
            assert!(!is_string(&ty), "non-string type should be rejected");
        }
    }
}
