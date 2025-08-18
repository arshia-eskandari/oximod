use crate::parsers::unwrap_option_type;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, Type};

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
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
    pub includes: Option<String>,
    pub alphanumeric: Option<bool>,
    pub multiple_of: Option<i64>,
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

/// Returns `true` if the type is a built-in numeric primitive (integer or float), otherwise `false`.
fn is_numeric(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            let seg = &tp.path.segments.first().unwrap().ident;
            matches!(
                seg.to_string().as_str(),
                "i8" | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "f32"
                    | "f64"
                    | "isize"
                    | "usize"
            )
        }
        _ => false,
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
    let field_name_str: &str = stringify!(#field_ident);

    let opt_inner = unwrap_option_type(field_ty);
    let is_optional = opt_inner.is_some();
    let inner_ty = opt_inner.unwrap_or(field_ty);

    // Final output for this field
    let mut checks: Vec<TokenStream> = Vec::new();

    // Per-field accumulators:
    // - compile-time failures
    let mut compile_errors: Vec<TokenStream> = Vec::new();
    // - rules that need a bound `val`
    let mut field_rules_val: Vec<TokenStream> = Vec::new();
    // - rules that directly access self.#field_ident (no `val`)
    let mut field_rules_direct: Vec<TokenStream> = Vec::new();

    // Type gates
    let is_str = is_string(inner_ty);
    let is_num = is_numeric(inner_ty);

    // --- required ---
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

    // --- min_length ---
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

    // --- max_length ---
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

    // --- email ---
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

    // --- pattern (std-only stable OnceLock<Result<Regex, Error>>) ---
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

    // --- non_empty ---
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

    // --- positive ---
    if matches!(positive_option, Some(true))
        && is_type_safe!(
            is_num,
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

    // --- negative ---
    if matches!(negative_option, Some(true))
        && is_type_safe!(
            is_num,
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

    // --- non_negative ---
    if matches!(non_negative_option, Some(true))
        && is_type_safe!(
            is_num,
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

    // --- min ---
    if let Some(min) = min_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(min)]` can only be applied to numeric fields"
        ) {
            field_rules_val.push(quote! {
                if (*val as i64) < #min {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be at least {}", #field_name_str, #min)
                        ),
                        &format!(
                            "Ensure '{}' is at least {}", stringify!(#field_ident), #min
                        )
                    ));
                }
            });
        }
    }

    // --- max ---
    if let Some(max) = max_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(max)]` can only be applied to numeric fields"
        ) {
            field_rules_val.push(quote! {
                if (*val as i64) > #max {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be at most {}", #field_name_str, #max)
                        ),
                        &format!(
                            "Ensure '{}' is at most {}", stringify!(#field_ident), #max
                        )
                    ));
                }
            });
        }
    }

    // --- starts_with ---
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

    // --- ends_with ---
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

    // --- includes ---
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

    // --- alphanumeric (ASCII fast path) ---
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

    // --- multiple_of ---
    if let Some(multiple) = multiple_of_option {
        if is_type_safe!(
            is_num,
            checks,
            field_ident,
            "`#[validate(multiple_of)]` can only be applied to numeric fields"
        ) {
            field_rules_val.push(quote! {
                if (*val as i64) % #multiple != 0 {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ValidationError(
                            format!("Field '{}' must be a multiple of {}", #field_name_str, #multiple)
                        ),
                        &format!(
                            "Ensure '{}' is a multiple of {}", stringify!(#field_ident), #multiple
                        )
                    ));
                }
            });
        }
    }

    // ---------- EMIT ONCE PER FIELD ----------

    // 1) compile-time failures
    checks.extend(compile_errors);

    // 2) rules that operate directly on self.#field_ident
    if !field_rules_direct.is_empty() {
        checks.push(quote! { { #(#field_rules_direct)* } });
    }

    // 3) rules that need a bound `val` — wrap ONCE with opt_check!
    if !field_rules_val.is_empty() {
        let grouped = opt_check!(is_optional, field_ident, {
            // (optional) bind a readable name once; zero-cost in release
            #(#field_rules_val)*
        });
        checks.push(grouped);
    }

    checks
}
