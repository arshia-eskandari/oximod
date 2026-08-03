//! Primitive numeric-type classification for validation.
//!
//! Numeric validation rules need to distinguish:
//!
//! - signed numbers, which support positive and negative checks;
//! - integers, which support `multiple_of`;
//! - all numeric primitives, which support minimum and maximum bounds.
//!
//! `Option<T>` fields are classified using their inner type.

use syn::Type;

use crate::validate::args::PrimitiveNum;

/// Returns whether a primitive numeric type can represent negative values.
#[inline]
pub fn is_signed(primitive: &PrimitiveNum) -> bool {
    use PrimitiveNum::*;

    matches!(primitive, I8 | I16 | I32 | I64 | I128 | Isize | F32 | F64)
}

/// Returns whether a primitive numeric type is an integer.
#[inline]
pub fn is_integer(primitive: &PrimitiveNum) -> bool {
    use PrimitiveNum::*;

    matches!(
        primitive,
        I8 | I16 | I32 | I64 | I128 | Isize | U8 | U16 | U32 | U64 | U128 | Usize
    )
}

/// Returns whether the classified type is numeric.
#[inline]
pub fn is_numeric(primitive: &PrimitiveNum) -> bool {
    primitive != &PrimitiveNum::NonNumeric
}

/// Classifies a Rust field type as a supported primitive numeric type.
///
/// A simple `Option<T>` is classified using its inner type. All unsupported
/// field types return [`PrimitiveNum::NonNumeric`].
#[inline]
pub fn primitive_of(ty: &Type) -> PrimitiveNum {
    use PrimitiveNum::*;

    let inner = crate::parsers::unwrap_option_type(ty).unwrap_or(ty);

    let Type::Path(type_path) = inner else {
        return NonNumeric;
    };

    let Some(segment) = type_path.path.segments.last() else {
        return NonNumeric;
    };

    let ident = &segment.ident;

    if ident == "i8" {
        I8
    } else if ident == "i16" {
        I16
    } else if ident == "i32" {
        I32
    } else if ident == "i64" {
        I64
    } else if ident == "i128" {
        I128
    } else if ident == "isize" {
        Isize
    } else if ident == "u8" {
        U8
    } else if ident == "u16" {
        U16
    } else if ident == "u32" {
        U32
    } else if ident == "u64" {
        U64
    } else if ident == "u128" {
        U128
    } else if ident == "usize" {
        Usize
    } else if ident == "f32" {
        F32
    } else if ident == "f64" {
        F64
    } else {
        NonNumeric
    }
}

#[cfg(test)]
mod tests {
    use syn::{Type, parse_quote, parse_str};

    use crate::validate::args::PrimitiveNum;

    use super::{is_integer, is_numeric, is_signed, primitive_of};

    #[test]
    fn classifies_all_supported_numeric_primitives() {
        let cases = [
            ("i8", PrimitiveNum::I8),
            ("i16", PrimitiveNum::I16),
            ("i32", PrimitiveNum::I32),
            ("i64", PrimitiveNum::I64),
            ("i128", PrimitiveNum::I128),
            ("isize", PrimitiveNum::Isize),
            ("u8", PrimitiveNum::U8),
            ("u16", PrimitiveNum::U16),
            ("u32", PrimitiveNum::U32),
            ("u64", PrimitiveNum::U64),
            ("u128", PrimitiveNum::U128),
            ("usize", PrimitiveNum::Usize),
            ("f32", PrimitiveNum::F32),
            ("f64", PrimitiveNum::F64),
        ];

        for (source, expected) in cases {
            let ty = parse_str::<Type>(source).expect("primitive type should parse");

            assert_eq!(
                primitive_of(&ty),
                expected,
                "unexpected classification for {source}"
            );
        }
    }

    #[test]
    fn classifies_optional_numeric_inner_type() {
        let ty: Type = parse_quote! {
            Option<i64>
        };

        assert_eq!(primitive_of(&ty), PrimitiveNum::I64);
    }

    #[test]
    fn non_numeric_types_are_classified_as_non_numeric() {
        let types: [Type; 4] = [
            parse_quote!(String),
            parse_quote!(Vec<i32>),
            parse_quote!(&i32),
            parse_quote!([i32; 3]),
        ];

        for ty in types {
            assert_eq!(primitive_of(&ty), PrimitiveNum::NonNumeric);
        }
    }

    #[test]
    fn signed_classification_includes_signed_integers_and_floats() {
        let signed = [
            PrimitiveNum::I8,
            PrimitiveNum::I16,
            PrimitiveNum::I32,
            PrimitiveNum::I64,
            PrimitiveNum::I128,
            PrimitiveNum::Isize,
            PrimitiveNum::F32,
            PrimitiveNum::F64,
        ];

        for primitive in signed {
            assert!(is_signed(&primitive), "{primitive:?} should be signed");
        }

        assert!(!is_signed(&PrimitiveNum::U32));
        assert!(!is_signed(&PrimitiveNum::NonNumeric));
    }

    #[test]
    fn integer_classification_excludes_floats() {
        assert!(is_integer(&PrimitiveNum::I32));
        assert!(is_integer(&PrimitiveNum::U64));

        assert!(!is_integer(&PrimitiveNum::F32));
        assert!(!is_integer(&PrimitiveNum::F64));
        assert!(!is_integer(&PrimitiveNum::NonNumeric));
    }

    #[test]
    fn numeric_classification_excludes_only_non_numeric() {
        assert!(is_numeric(&PrimitiveNum::I32));
        assert!(is_numeric(&PrimitiveNum::U32));
        assert!(is_numeric(&PrimitiveNum::F64));

        assert!(!is_numeric(&PrimitiveNum::NonNumeric));
    }
}
