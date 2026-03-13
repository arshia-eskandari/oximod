use crate::validate::args::PrimitiveNum;

#[inline]
pub fn is_signed(prim: &PrimitiveNum) -> bool {
    use PrimitiveNum::*;
    matches!(prim, I8 | I16 | I32 | I64 | I128 | Isize | F32 | F64)
}

#[inline]
pub fn is_integer(prim: &PrimitiveNum) -> bool {
    use PrimitiveNum::*;
    matches!(
        prim,
        I8 | I16 | I32 | I64 | I128 | Isize | U8 | U16 | U32 | U64 | U128 | Usize
    )
}

#[inline]
pub fn is_numeric(prim: &PrimitiveNum) -> bool {
    prim != &PrimitiveNum::NonNumeric
}

#[inline]
pub fn primitive_of(ty: &syn::Type) -> PrimitiveNum {
    use PrimitiveNum::*;
    let inner = crate::parsers::unwrap_option_type(ty).unwrap_or(ty);

    let syn::Type::Path(tp) = inner else {
        return NonNumeric;
    };
    let Some(seg) = tp.path.segments.last() else {
        return NonNumeric;
    };
    let id = &seg.ident;

    if id == "i8" {
        I8
    } else if id == "i16" {
        I16
    } else if id == "i32" {
        I32
    } else if id == "i64" {
        I64
    } else if id == "i128" {
        I128
    } else if id == "isize" {
        Isize
    } else if id == "u8" {
        U8
    } else if id == "u16" {
        U16
    } else if id == "u32" {
        U32
    } else if id == "u64" {
        U64
    } else if id == "u128" {
        U128
    } else if id == "usize" {
        Usize
    } else if id == "f32" {
        F32
    } else if id == "f64" {
        F64
    } else {
        NonNumeric
    }
}

