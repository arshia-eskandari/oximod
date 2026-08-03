//! Classification of field types that support length validation.
//!
//! Length-based rules such as `min_length`, `max_length`, and `non_empty`
//! support strings and common collection types.
//!
//! Simple `Option<T>` fields are classified using their inner type.

use syn::Type;

use super::type_checks::{
    is_array, is_btreemap, is_btreeset, is_hashmap, is_hashset, is_string, is_vec, is_vecdeque,
};

/// Identifies the length operation supported by a field type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthKind {
    /// A string-like value.
    String,

    /// A `Vec<T>`.
    Vec,

    /// A fixed-size array.
    Array,

    /// A `VecDeque<T>`.
    VecDeque,

    /// A `HashSet<T>`.
    HashSet,

    /// A `BTreeSet<T>`.
    BTreeSet,

    /// A `HashMap<K, V>`.
    HashMap,

    /// A `BTreeMap<K, V>`.
    BTreeMap,
}

/// Classifies a field type that supports length validation.
///
/// A simple `Option<T>` is classified using its inner type. Unsupported types
/// return `None`.
#[inline]
pub fn length_kind_of(ty: &Type) -> Option<LengthKind> {
    let inner = crate::parsers::unwrap_option_type(ty).unwrap_or(ty);

    if is_string(inner) {
        Some(LengthKind::String)
    } else if is_vec(inner) {
        Some(LengthKind::Vec)
    } else if is_array(inner) {
        Some(LengthKind::Array)
    } else if is_vecdeque(inner) {
        Some(LengthKind::VecDeque)
    } else if is_hashset(inner) {
        Some(LengthKind::HashSet)
    } else if is_btreeset(inner) {
        Some(LengthKind::BTreeSet)
    } else if is_hashmap(inner) {
        Some(LengthKind::HashMap)
    } else if is_btreemap(inner) {
        Some(LengthKind::BTreeMap)
    } else {
        None
    }
}

/// Returns whether a field type supports length validation.
#[inline]
pub fn is_length_type(ty: &Type) -> bool {
    length_kind_of(ty).is_some()
}

#[cfg(test)]
mod tests {
    use syn::{Type, parse_quote, parse_str};

    use super::{LengthKind, is_length_type, length_kind_of};

    #[test]
    fn classifies_all_supported_length_types() {
        let cases = [
            ("String", LengthKind::String),
            ("&str", LengthKind::String),
            ("Cow<'static, str>", LengthKind::String),
            ("Vec<u8>", LengthKind::Vec),
            ("[u8; 4]", LengthKind::Array),
            ("VecDeque<u8>", LengthKind::VecDeque),
            ("HashSet<u8>", LengthKind::HashSet),
            ("BTreeSet<u8>", LengthKind::BTreeSet),
            ("HashMap<String, u8>", LengthKind::HashMap),
            ("BTreeMap<String, u8>", LengthKind::BTreeMap),
        ];

        for (source, expected) in cases {
            let ty = parse_str::<Type>(source).expect("length-capable type should parse");

            assert_eq!(
                length_kind_of(&ty),
                Some(expected),
                "unexpected classification for {source}"
            );
        }
    }

    #[test]
    fn recognizes_qualified_collection_paths() {
        let cases = [
            ("std::vec::Vec<u8>", LengthKind::Vec),
            ("std::collections::VecDeque<u8>", LengthKind::VecDeque),
            ("std::collections::HashSet<u8>", LengthKind::HashSet),
            ("std::collections::BTreeSet<u8>", LengthKind::BTreeSet),
            ("std::collections::HashMap<String, u8>", LengthKind::HashMap),
            (
                "std::collections::BTreeMap<String, u8>",
                LengthKind::BTreeMap,
            ),
        ];

        for (source, expected) in cases {
            let ty = parse_str::<Type>(source).expect("qualified collection type should parse");

            assert_eq!(
                length_kind_of(&ty),
                Some(expected),
                "unexpected classification for {source}"
            );
        }
    }

    #[test]
    fn classifies_optional_inner_types() {
        let string: Type = parse_quote! {
            Option<String>
        };
        let vector: Type = parse_quote! {
            Option<Vec<u8>>
        };
        let array: Type = parse_quote! {
            Option<[u8; 4]>
        };

        assert_eq!(length_kind_of(&string), Some(LengthKind::String));
        assert_eq!(length_kind_of(&vector), Some(LengthKind::Vec));
        assert_eq!(length_kind_of(&array), Some(LengthKind::Array));
    }

    #[test]
    fn rejects_types_without_length_support() {
        let types: [Type; 5] = [
            parse_quote!(i32),
            parse_quote!(bool),
            parse_quote!((String, String)),
            parse_quote!(CustomType),
            parse_quote!(Option<i32>),
        ];

        for ty in types {
            assert_eq!(length_kind_of(&ty), None);
            assert!(!is_length_type(&ty));
        }
    }

    #[test]
    fn is_length_type_matches_classification() {
        let supported: Type = parse_quote! {
            Vec<String>
        };
        let unsupported: Type = parse_quote! {
            u64
        };

        assert!(is_length_type(&supported));
        assert!(!is_length_type(&unsupported));
    }
}
