/// Usage:
/// parse_lit_to_primitive_type!(lit, i32)              -> Result<i32,  syn::Error>
/// parse_lit_to_primitive_type!(&lit, String, "msg")   -> Result<String, syn::Error>
macro_rules! parse_lit_to_primitive_type {
    // 3-arg form with custom error message
    ($lit:expr, $ty:ty, $msg:expr) => {{
        // Support passing either `Lit` or `&Lit`
        let __lit: &::syn::Lit = &$lit;

        match __lit {
            ::syn::Lit::Str(s) => s.value().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(s), format!("{}: {}", $msg, e))
            }),
            ::syn::Lit::Int(i) => i.base10_digits().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(i), format!("{}: {}", $msg, e))
            }),
            ::syn::Lit::Float(f) => f.base10_digits().parse::<$ty>().map_err(|e| {
                ::syn::Error::new(::syn::spanned::Spanned::span(f), format!("{}: {}", $msg, e))
            }),
            other => ::core::result::Result::Err(::syn::Error::new(
                ::syn::spanned::Spanned::span(other),
                $msg,
            )),
        }
    }};

    // 2-arg form with a default message
    ($lit:expr, $ty:ty) => {{
        parse_lit_to_primitive_type!(
            $lit,
            $ty,
            concat!(
                "expected a literal compatible with type `",
                stringify!($ty),
                "`"
            )
        )
    }};
}

pub(crate) use parse_lit_to_primitive_type;
