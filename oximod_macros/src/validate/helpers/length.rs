use super::type_checks::{
    is_array, is_btreemap, is_btreeset, is_hashmap, is_hashset, is_string, is_vec, is_vecdeque,
};
use syn::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthKind {
    String,
    Vec,
    Array,
    VecDeque,
    HashSet,
    BTreeSet,
    HashMap,
    BTreeMap,
}

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

pub fn is_length_type(ty: &Type) -> bool {
    length_kind_of(ty).is_some()
}
