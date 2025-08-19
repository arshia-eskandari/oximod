use crate::parsers::{parse_f64_for_range, parse_u128_for_range, unwrap_option_type};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, LitFloat, LitInt, Type};

#[derive(Debug)]
pub enum LitNum {
    Int { lit: syn::LitInt, neg: bool },
    Float { lit: syn::LitFloat, neg: bool },
}

#[derive(Default, Debug)]
/// Arguments for field validation in OxiMod using the `#[validate(...)]` attribute.
///
/// This struct is populated from the `#[validate(...)]` attribute
/// and specifies the set of validation rules to apply to the field.
///
/// # Fields
///
/// - `min_length`: (Optional) Minimum allowed length for strings.
///   - The field’s length must be >= this value.
///   - Default: no minimum‐length constraint.
///
/// - `max_length`: (Optional) Maximum allowed length for strings.
///   - The field’s length must be <= this value.
///   - Default: no maximum‐length constraint.
///
/// - `required`: (Optional) Whether the field is required (i.e., must be present and non-`None`).
///   - If `true`, an error is returned when the field is missing or `None`.
///   - Default: `false` (field may be omitted).
///
/// - `email`: (Optional) Whether the field must be a valid email address.
///   - If `true`, the field’s string value is matched against a basic email regex.
///   - Default: `false` (no email format check).
///
/// - `pattern`: (Optional) A custom regular expression that the field’s string value must match.
///   - If provided, the field’s string must match this regex exactly.
///   - Default: no custom pattern enforced.
///
/// - `non_empty`: (Optional) Whether the field’s string value must not be empty (`""`).
///   - If `true`, empty strings are rejected.
///   - Default: `false` (empty strings allowed).
///
/// - `positive`: (Optional) Whether the field’s numeric value must be strictly > 0.
///   - If `true`, zero and negative values are rejected.
///   - Default: `false` (no positivity constraint).
///
/// - `negative`: (Optional) Whether the field’s numeric value must be strictly < 0.
///   - If `true`, zero and positive values are rejected.
///   - Default: `false` (no negativity constraint).
///
/// - `non_negative`: (Optional) Whether the field’s numeric value must be >= 0.
///   - If `true`, negative values are rejected.
///   - Default: `false` (no non-negative constraint).
///
/// - `min`: (Optional) Minimum allowed value for numeric fields (inclusive).
///   - If provided, the field’s numeric value must be >= this value.
///   - Default: no minimum‐value constraint.
///
/// - `max`: (Optional) Maximum allowed value for numeric fields (inclusive).
///   - If provided, the field’s numeric value must be <= this value.
///   - Default: no maximum‐value constraint.
///
/// - `starts_with`: (Optional) A required string prefix the field value must start with.
///   - If provided, the field value must begin with this substring.
///   - Default: not enforced.
///
/// - `ends_with`: (Optional) A required string suffix the field value must end with.
///   - If provided, the field value must end with this substring.
///   - Default: not enforced.
///
/// - `includes`: (Optional) A required substring the field value must contain.
///   - If provided, the field must contain this substring somewhere.
///   - Default: not enforced.
///
/// - `alphanumeric`: (Optional) Whether the field must only contain letters and digits.
///   - If `true`, the field value must match `/^[a-zA-Z0-9]+$/`.
///   - Default: `false` (no alphanumeric constraint).
///
/// - `multiple_of`: (Optional) Whether the field value must be a multiple of the given integer.
///   - If provided, the field value must be evenly divisible by this number.
///   - Default: not enforced.
///
/// # Example
///
/// ```rust
/// #[derive(Validate)]
/// struct User {
///     #[validate(
///         required = true,
///         min_length = 3,
///         max_length = 30,
///         pattern = r"^[a-zA-Z0-9_]+$"
///     )]
///     username: String,
///
///     #[validate(email)]
///     contact_email: Option<String>,
///
///     #[validate(non_negative = true, max = 100)]
///     score: i64,
/// }
/// ```
pub struct ValidateArgs {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub required: Option<bool>,
    pub email: Option<bool>,
    pub pattern: Option<String>,
    pub non_empty: Option<bool>,
    pub positive: Option<bool>,
    pub negative: Option<bool>,
    pub non_negative: Option<bool>,
    pub min: Option<LitNum>,
    pub max: Option<LitNum>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
    pub includes: Option<String>,
    pub alphanumeric: Option<bool>,
    pub multiple_of: Option<syn::LitInt>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PrimitiveNum {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    NonNumeric,
}

fn primitive_of(ty: &syn::Type) -> PrimitiveNum {
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

macro_rules! opt_check {
    (
        $is_opt:expr,
        $field:ident,
        $($check:tt)*
    ) => {
        {
        if $is_opt {
            quote! {
                if let Some(val) = &self.#$field {
                    $($check)*
                }
            }
        } else {
            quote! {
                {
                    let val = &self.#$field;
                    $($check)*
                }
            }
        }
        }
    };
}

macro_rules! is_type_safe {
    ($cond:expr, $checks:expr, $field_ident:expr, $msg:expr) => {{
        if !$cond {
            $checks.push(quote_spanned! { $field_ident.span() =>
                compile_error!($msg);
            });
            false
        } else {
            true
        }
    }};
}

/// Returns `true` if the type is a string-like type (`String` or `&str`), otherwise `false`.
fn is_string(ty: &Type) -> bool {
    match ty {
        syn::Type::Path(tp) => tp.path.is_ident("String"),
        syn::Type::Reference(r) => {
            matches!(&*r.elem, syn::Type::Path(tp) if tp.path.is_ident("str"))
        }
        _ => false,
    }
}

fn is_numeric(prim: &PrimitiveNum) -> bool {
    prim != &PrimitiveNum::NonNumeric
}

fn emit_int_lit(lit: &LitInt, neg: bool) -> TokenStream {
    if neg {
        quote! { - #lit }
    } else {
        quote! { #lit }
    }
}

fn emit_float_from_float(lit: &LitFloat, neg: bool) -> TokenStream {
    if neg {
        quote! { - #lit }
    } else {
        quote! { #lit }
    }
}

fn emit_float_from_int(lit: &LitInt, neg: bool) -> TokenStream {
    let s = {
        let mut s = String::from(lit.base10_digits());
        if !s.contains('.') {
            s.push_str(".0");
        }
        s
    };
    let lf = LitFloat::new(&s, lit.span());
    if neg {
        quote! { - #lf }
    } else {
        quote! { #lf }
    }
}

fn check_int_fits_primitive(
    span: Span,
    neg: bool,
    mag: u128,
    prim: PrimitiveNum,
) -> Option<TokenStream> {
    use PrimitiveNum::*;
    let err = |msg: &str| Some(quote_spanned! { span => compile_error!(#msg); });

    let fits_signed = |min: i128, max: i128| -> bool {
        if neg {
            let m = match i128::try_from(mag) {
                Ok(v) => v,
                Err(_) => return false,
            };
            -m >= min && -m <= max
        } else {
            let m = match i128::try_from(mag) {
                Ok(v) => v,
                Err(_) => return false,
            };
            m >= min && m <= max
        }
    };
    let fits_unsigned = |max: u128| -> bool {
        if neg {
            return false;
        }
        mag <= max
    };

    match prim {
        I8 => {
            if !fits_signed(i8::MIN as i128, i8::MAX as i128) {
                return err("numeric bound does not fit `i8`");
            }
        }
        I16 => {
            if !fits_signed(i16::MIN as i128, i16::MAX as i128) {
                return err("numeric bound does not fit `i16`");
            }
        }
        I32 => {
            if !fits_signed(i32::MIN as i128, i32::MAX as i128) {
                return err("numeric bound does not fit `i32`");
            }
        }
        I64 => {
            if !fits_signed(i64::MIN as i128, i64::MAX as i128) {
                return err("numeric bound does not fit `i64`");
            }
        }
        I128 => {
            if neg {
                if mag > (i128::MAX as u128) + 1 {
                    return err("numeric bound does not fit `i128`");
                }
            } else if mag > i128::MAX as u128 {
                return err("numeric bound does not fit `i128`");
            }
        }
        Isize => { /* let rustc enforce target width */ }

        U8 => {
            if !fits_unsigned(u8::MAX as u128) {
                return err("numeric bound does not fit `u8`");
            }
        }
        U16 => {
            if !fits_unsigned(u16::MAX as u128) {
                return err("numeric bound does not fit `u16`");
            }
        }
        U32 => {
            if !fits_unsigned(u32::MAX as u128) {
                return err("numeric bound does not fit `u32`");
            }
        }
        U64 => {
            if !fits_unsigned(u64::MAX as u128) {
                return err("numeric bound does not fit `u64`");
            }
        }
        U128 => {
            if neg {
                return err("negative bound not allowed for unsigned type");
            }
        }
        Usize => { /* let rustc enforce target width */ }

        F32 | F64 | PrimitiveNum::NonNumeric => {}
    }
    None
}

fn check_float_fits_primitive(span: Span, v: f64, prim: PrimitiveNum) -> Option<TokenStream> {
    use PrimitiveNum::*;
    let err = |msg: &str| Some(quote_spanned! { span => compile_error!(#msg); });

    if !v.is_finite() {
        return err("float bound must be finite");
    }
    match prim {
        F32 => {
            if v < f32::MIN as f64 || v > f32::MAX as f64 {
                return err("float bound does not fit `f32`");
            }
        }
        F64 => { /* any finite f64 is OK */ }
        _ => {}
    }
    None
}

fn is_signed(prim: PrimitiveNum) -> bool {
    use PrimitiveNum::*;
    matches!(prim, I8 | I16 | I32 | I64 | I128 | Isize | F32 | F64)
}

/// Produce a RHS numeric token appropriate for the field's `prim` type from a `LitNum` bound.
/// - Emits compile_error! into `compile_errors` if the field is non-numeric,
///   or the literal doesn't fit the field's primitive.
/// - Returns Some(rhs_tokens) if OK, None if a compile_error! was emitted.
///
/// Requirements:
/// - `emit_int_lit`, `emit_float_from_int`, `emit_float_from_float`
/// - `parse_u128_for_range`, `parse_f64_for_range`
/// - `check_int_fits_primitive`, `check_float_fits_primitive`
/// - `PrimitiveNum` enum + `primitive_of(...)` already in your module
fn rhs_for_numeric_bound(
    prim: PrimitiveNum,
    bound: &LitNum,
    field_ident: &Ident,
    compile_errors: &mut Vec<TokenStream>,
) -> Option<TokenStream> {
    match (prim, bound) {
        // Non-numeric fields
        (PrimitiveNum::NonNumeric, _) => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(min)]`/`max` can only be applied to numeric fields");
            });
            None
        }

        // Integer fields + integer literal
        (
            PrimitiveNum::I8
            | PrimitiveNum::I16
            | PrimitiveNum::I32
            | PrimitiveNum::I64
            | PrimitiveNum::I128
            | PrimitiveNum::Isize
            | PrimitiveNum::U8
            | PrimitiveNum::U16
            | PrimitiveNum::U32
            | PrimitiveNum::U64
            | PrimitiveNum::U128
            | PrimitiveNum::Usize,
            &LitNum::Int { ref lit, neg },
        ) => {
            if let Ok(mag) = parse_u128_for_range(lit) {
                if let Some(err) = check_int_fits_primitive(lit.span(), neg, mag, prim) {
                    compile_errors.push(err);
                    return None;
                }
            }
            Some(emit_int_lit(lit, neg))
        }

        // ❗ Integer fields + float literal → reject
        (
            PrimitiveNum::I8
            | PrimitiveNum::I16
            | PrimitiveNum::I32
            | PrimitiveNum::I64
            | PrimitiveNum::I128
            | PrimitiveNum::Isize
            | PrimitiveNum::U8
            | PrimitiveNum::U16
            | PrimitiveNum::U32
            | PrimitiveNum::U64
            | PrimitiveNum::U128
            | PrimitiveNum::Usize,
            &LitNum::Float { lit: _, .. },
        ) => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("float literal is not allowed for integer field in `#[validate(min)]`/`max`");
            });
            None
        }

        // Float fields + integer literal → emit as float
        (PrimitiveNum::F32 | PrimitiveNum::F64, &LitNum::Int { ref lit, neg }) => {
            Some(emit_float_from_int(lit, neg))
        }

        // Float fields + float literal
        (PrimitiveNum::F32 | PrimitiveNum::F64, &LitNum::Float { ref lit, neg }) => {
            if matches!(prim, PrimitiveNum::F32) {
                if let Ok(v64) = parse_f64_for_range(lit) {
                    let signed = if neg { -v64 } else { v64 };
                    if let Some(err) = check_float_fits_primitive(lit.span(), signed, prim) {
                        compile_errors.push(err);
                        return None;
                    }
                }
            }
            Some(emit_float_from_float(lit, neg))
        }
    }
}

/// Produce a RHS integer token for `multiple_of` (integers only).
/// - Emits compile_error! if field is float or non-numeric, or literal doesn't fit.
/// - Returns Some(rhs) if OK, None if error.
fn rhs_for_integer_multiple_of(
    prim: PrimitiveNum,
    lit: &LitInt,
    field_ident: &Ident,
    compile_errors: &mut Vec<TokenStream>,
) -> Option<TokenStream> {
    match prim {
        PrimitiveNum::F32 | PrimitiveNum::F64 => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(multiple_of)]` is not allowed on float fields");
            });
            None
        }
        PrimitiveNum::NonNumeric => {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!("`#[validate(multiple_of)]` can only be applied to integer fields");
            });
            None
        }
        _ => {
            if let Ok(mag) = parse_u128_for_range(lit) {
                if mag == 0 {
                    compile_errors.push(quote_spanned! { lit.span() =>
                        compile_error!("`multiple_of` must be non-zero");
                    });
                    return None;
                }
                if let Some(err) = check_int_fits_primitive(lit.span(), false, mag, prim) {
                    compile_errors.push(err);
                    return None;
                }
            }
            Some(quote! { #lit })
        }
    }
}

/// Generates validation `TokenStream`s for a field based on `ValidateArgs`,
/// producing compile-time and runtime checks appropriate to the field’s type.
pub fn generate_validate_model_tokens(
    struct_ident: &Ident,
    field_ident: &Ident,
    field_ty: &Type,
    validate_args: ValidateArgs,
) -> Vec<TokenStream> {
    let ValidateArgs {
        min_length: min_length_option,
        max_length: max_length_option,
        required,
        email: email_option,
        pattern: pattern_option,
        non_empty: non_empty_option,
        positive: positive_option,
        negative: negative_option,
        non_negative: non_negative_option,
        min: min_option,
        max: max_option,
        starts_with: starts_with_option,
        ends_with: ends_with_option,
        includes: includes_option,
        alphanumeric: alphanumeric_option,
        multiple_of: multiple_of_option,
    } = &validate_args;
    let field_name_str = field_ident.to_string();

    let opt_inner = unwrap_option_type(field_ty);
    let is_optional = opt_inner.is_some();
    let inner_ty = opt_inner.unwrap_or(field_ty);

    let mut checks: Vec<TokenStream> = Vec::new();

    let mut compile_errors: Vec<TokenStream> = Vec::new();
    let mut field_rules_val: Vec<TokenStream> = Vec::new();
    let mut field_rules_direct: Vec<TokenStream> = Vec::new();

    let is_str = is_string(inner_ty);
    let prim = primitive_of(inner_ty);
    let is_num = is_numeric(&prim);

    if matches!(required, Some(true)) {
        if !is_optional {
            compile_errors.push(quote_spanned! { field_ident.span() =>
                compile_error!(
                    concat!(
                        "Field '", stringify!(#field_ident),
                        "' cannot use #[validate(required)] because it is not Option<T>"
                    )
                );
            });
        } else {
            field_rules_direct.push(quote! {
                if self.#field_ident.is_none() {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' is required", #field_name_str)
                        ),
                        concat!("Provide a value for '", stringify!(#field_ident), "'")
                    ));
                }
            });
        }
    }

    if let Some(min_length) = min_length_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(min_length)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.len() < (#min_length as usize) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!(
                                "Field '{}' must be at least {} characters long",
                                #field_name_str,
                                #min_length
                            )
                        ),
                        &format!(
                            "Ensure '{}' has at least {} characters", stringify!(#field_ident), #min_length
                        )
                    ));
                }
            });
        }
    }

    if let Some(max_length) = max_length_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(max_length)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.len() > (#max_length as usize) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!(
                                "Field '{}' must be at most {} characters long",
                                #field_name_str,
                                #max_length
                            )
                        ),
                        &format!(
                            "Ensure '{}' has at most {} characters", stringify!(#field_ident), #max_length
                        )
                    ));
                }
            });
        }
    }

    if matches!(email_option, Some(true))
        && is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(email)]` can only be applied to string fields"
        )
    {
        field_rules_val.push(quote! {
            match val.split_once('@') {
                Some((local, domain)) if !local.is_empty() && !domain.is_empty() => {
                    match domain.rsplit_once('.') {
                        Some((lhs, rhs)) if !lhs.is_empty() && !rhs.is_empty() => { /* OK */ }
                        _ => {
                            return Err(::oximod::_attach_printables!(
                                ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                    format!("Field '{}' must be a valid email address", #field_name_str)
                                ),
                                concat!("Ensure '", stringify!(#field_ident), "' is in the format local@domain")
                            ));
                        }
                    }
                }
                _ => {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be a valid email address", #field_name_str)
                        ),
                        concat!("Ensure '", stringify!(#field_ident), "' is in the format local@domain")
                    ));
                }
            }
        });
    }

    if let Some(pattern) = pattern_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(pattern)]` can only be applied to string fields"
        ) {
            let upper_field_name = field_ident.to_string().to_uppercase();
            let upper_struct_name = struct_ident.to_string().to_uppercase();
            let re_ident = format_ident!("__OXIMOD_RE_{}_{}", upper_struct_name, upper_field_name);

            field_rules_val.push(quote! {
                static #re_ident: ::std::sync::OnceLock<
                    Result<::oximod::_regex::Regex, ::oximod::_regex::Error>
                > = ::std::sync::OnceLock::new();

                let regex = #re_ident
                    .get_or_init(|| ::oximod::_regex::Regex::new(#pattern))
                    .as_ref()
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!(
                                    "Invalid regex pattern in validation for '{}': {}",
                                    #field_name_str, e
                                )
                            ),
                            concat!("Check the regex pattern for '", stringify!(#field_ident), "'")
                        )
                    })?;

                if !regex.is_match(val) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!(
                                "Field '{}' does not match the required pattern",
                                #field_name_str
                            )
                        ),
                        &format!(
                            "Ensure '{}' matches regex {}", stringify!(#field_ident), #pattern
                        )
                    ));
                }
            });
        }
    }

    if let Some(true) = non_empty_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(non_empty)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if val.trim().is_empty() {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be non-empty", #field_name_str)
                        ),
                        concat!("Provide a non-empty string for '", stringify!(#field_ident), "'")
                    ));
                }
            });
        }
    }

    if matches!(positive_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(positive)]` can only be applied to numeric fields"
        )
    {
        field_rules_val.push(quote! {
            if *val <= 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be positive", #field_name_str)
                    ),
                    concat!("Use a positive value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if matches!(negative_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(negative)]` can only be applied to numeric fields"
        )
    {
        field_rules_val.push(quote! {
            if *val >= 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be negative", #field_name_str)
                    ),
                    concat!("Use a negative value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if matches!(non_negative_option, Some(true))
        && is_type_safe!(
            is_num && is_signed(prim),
            checks,
            field_ident,
            "`#[validate(non_negative)]` can only be applied to numeric fields"
        )
    {
        field_rules_val.push(quote! {
            if *val < 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be non-negative", #field_name_str)
                    ),
                    concat!("Use zero or a positive value for '", stringify!(#field_ident), "'")
                ));
            }
        });
    }

    if let Some(min) = min_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(min)]` can only be applied to numeric fields"
        ) {
            if let Some(rhs) = rhs_for_numeric_bound(prim, min, field_ident, &mut compile_errors) {
                field_rules_val.push(quote! {
                    if *val < #rhs {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!("Field '{}' must be at least {}", #field_name_str, #rhs)
                            ),
                            &format!("Ensure '{}' is at least {}", stringify!(#field_ident), #rhs)
                        ));
                    }
                });
            }
        }
    }

    if let Some(max) = max_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(max)]` can only be applied to numeric fields"
        ) {
            if let Some(rhs) = rhs_for_numeric_bound(prim, max, field_ident, &mut compile_errors) {
                field_rules_val.push(quote! {
                    if *val > #rhs {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ValidationError(
                                format!("Field '{}' must be at most {}", #field_name_str, #rhs)
                            ),
                            &format!("Ensure '{}' is at most {}", stringify!(#field_ident), #rhs)
                        ));
                    }
                });
            }
        }
    }

    if let Some(start) = starts_with_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(starts_with)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.starts_with(#start) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must start with '{}'", #field_name_str, #start)
                        ),
                        &format!(
                            "Ensure '{}' starts with {}", stringify!(#field_ident), #start
                        )
                    ));
                }
            });
        }
    }

    if let Some(end) = ends_with_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(ends_with)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.ends_with(#end) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must end with '{}'", #field_name_str, #end)
                        ),
                        &format!(
                            "Ensure '{}' ends with {}", stringify!(#field_ident), #end
                        )
                    ));
                }
            });
        }
    }

    if let Some(substr) = includes_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(includes)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.contains(#substr) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must include '{}'", #field_name_str, #substr)
                        ),
                        &format!(
                            "Ensure '{}' includes {}", stringify!(#field_ident), #substr
                        )
                    ));
                }
            });
        }
    }

    if let Some(true) = alphanumeric_option {
        if is_type_safe!(
            is_str,
            checks,
            field_ident,
            "`#[validate(alphanumeric)]` can only be applied to string fields"
        ) {
            field_rules_val.push(quote! {
                if !val.as_bytes().iter().all(|b| b.is_ascii_alphanumeric()) {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must contain only alphanumeric characters", #field_name_str)
                        ),
                        concat!("Ensure '", stringify!(#field_ident), "' has only letters and numbers")
                    ));
                }
            });
        }
    }

    if let Some(multiple) = multiple_of_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(multiple_of)]` can only be applied to numeric fields"
        ) {
            if let Some(rhs) =
                rhs_for_integer_multiple_of(prim, multiple, field_ident, &mut compile_errors)
            {
                field_rules_val.push(quote! {
            if (*val % #rhs) != 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OxiModError::ValidationError(
                        format!("Field '{}' must be a multiple of {}", #field_name_str, #rhs)
                    ),
                    &format!("Ensure '{}' is divisible by {}", stringify!(#field_ident), #rhs)
                ));
            }
        });
            }
        }
    }

    checks.extend(compile_errors);

    if !field_rules_direct.is_empty() {
        checks.push(quote! { { #(#field_rules_direct)* } });
    }

    if !field_rules_val.is_empty() {
        let grouped = opt_check!(is_optional, field_ident, {
            #(#field_rules_val)*
        });
        checks.push(grouped);
    }

    checks
}
