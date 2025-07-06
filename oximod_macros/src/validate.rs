use proc_macro2::TokenStream;
use quote::quote;
use syn::{ Type, Attribute, GenericArgument, Lit, PathArguments, Ident };

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

pub fn unwrap_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = generic_args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

pub fn parse_validate_args(attr: &Attribute) -> syn::Result<ValidateArgs> {
    let mut args = ValidateArgs::default();

    if attr.path().is_ident("validate") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("min_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `min_length`")
                    );
                }
            } else if meta.path.is_ident("max_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `max_length`")
                    );
                }
            } else if meta.path.is_ident("required") {
                args.required = Some(true);
            } else if meta.path.is_ident("email") {
                args.email = Some(true);
            } else if meta.path.is_ident("pattern") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.pattern = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `pattern`")
                    );
                }
            } else if meta.path.is_ident("non_empty") {
                args.non_empty = Some(true);
            } else if meta.path.is_ident("positive") {
                args.positive = Some(true);
            } else if meta.path.is_ident("negative") {
                args.negative = Some(true);
            } else if meta.path.is_ident("non_negative") {
                args.non_negative = Some(true);
            } else if meta.path.is_ident("min") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(lit.span(), "expected integer literal for `min`"));
                }
            } else if meta.path.is_ident("max") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(lit.span(), "expected integer literal for `max`"));
                }
            } else if meta.path.is_ident("starts_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.starts_with = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected string literal for `starts_with`")
                    );
                }
            } else if meta.path.is_ident("ends_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.ends_with = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected string literal for `ends_with`")
                    );
                }
            } else if meta.path.is_ident("includes") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.includes = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected string literal for `includes`")
                    );
                }
            } else if meta.path.is_ident("alphanumeric") {
                args.alphanumeric = Some(true);
            } else if meta.path.is_ident("multiple_of") {
                let lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = &lit {
                    let val = lit_int.base10_parse::<i64>()?;
                    if val == 0 {
                        return Err(
                            syn::Error::new(lit.span(), "`multiple_of` must be greater than 0")
                        );
                    }
                    args.multiple_of = Some(val);
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `multiple_of`")
                    );
                }
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(args)
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

pub fn generate_validate_model_tokens(
    field_ident: &Ident,
    field_ty: &Type,
    validate_args: ValidateArgs
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
    let is_optional = unwrap_option_type(field_ty).is_some();
    let mut checks = vec![];

    if let Some(min_length) = min_length_option {
        let inner =
            quote! {
            if val.len() < (#min_length as usize) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be at least {} characters long",
                                stringify!(#field_ident),
                                #min_length
                        )
                    ),
                    concat!("Ensure '", stringify!(#field_ident),
                           "' has at least ", #min_length, " characters.")
                ));
            }
        };

        let snippet = opt_check!(is_optional, field_ident, #inner);

        checks.push(snippet);
    }

    if let Some(max_length) = max_length_option {
        let inner = quote! {
            if val.len() > (#max_length as usize) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be at most {} characters long",
                                stringify!(#field_ident),
                                #max_length
                        )
                    ),
                    concat!("Ensure '", stringify!(#field_ident),
                           "' has at most ", #max_length, " characters.")
                ));
            }
        };
    
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    

    if let Some(req) = required {
        if *req && is_optional {
            checks.push(
                quote! {
                    if self.#field_ident.is_none() {
                        return Err(::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ValidationError(
                                format!("Field '{}' is required", stringify!(#field_ident))
                            ),
                            concat!("Provide a value for '", stringify!(#field_ident), "'.")
                        ));
                    }
                }
            );
        }
    }
    

    if let Some(is_email) = email_option {
        if *is_email {
            let inner = quote! {
                if !val.contains('@') || !val.contains('.') {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be a valid email address", stringify!(#field_ident))
                        ),
                        concat!("Provide a valid email for '", stringify!(#field_ident), "'.")
                    ));
                }
    
                let parts: Vec<&str> = val.split('@').collect();
                if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be a valid email address", stringify!(#field_ident))
                        ),
                        concat!("Ensure '", stringify!(#field_ident), "' is in the format local@domain.")
                    ));
                }
            };
    
            let snippet = opt_check!(is_optional, field_ident, #inner);
            checks.push(snippet);
        }
    }
    

    if let Some(pattern) = pattern_option {
        let inner = quote! {
            let regex = ::oximod::_regex::Regex::new(#pattern).map_err(|e| {
                ::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Invalid regex pattern in validation for '{}': {}", stringify!(#field_ident), e)
                    ),
                    concat!("Check the regex pattern for '", stringify!(#field_ident), "'.")
                )
            })?;
    
            if !regex.is_match(val) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' does not match the required pattern", stringify!(#field_ident))
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' matches regex: ", #pattern, ".")
                ));
            }
        };
    
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    

    if let Some(true) = non_empty_option {
        let inner = quote! {
            if val.trim().is_empty() {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be non-empty", stringify!(#field_ident))
                    ),
                    concat!("Provide a non-empty string for '", stringify!(#field_ident), "'.")
                ));
            }
        };
    
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    

    if let Some(positive) = positive_option {
        if *positive {
            let inner = quote! {
                if *val <= 0 {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be positive", stringify!(#field_ident))
                        ),
                        concat!("Use a positive value for '", stringify!(#field_ident), "'.")
                    ));
                }
            };
            let snippet = opt_check!(is_optional, field_ident, #inner);
            checks.push(snippet);
        }
    }
    
    if let Some(negative) = negative_option {
        if *negative {
            let inner = quote! {
                if *val >= 0 {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be negative", stringify!(#field_ident))
                        ),
                        concat!("Use a negative value for '", stringify!(#field_ident), "'.")
                    ));
                }
            };
            let snippet = opt_check!(is_optional, field_ident, #inner);
            checks.push(snippet);
        }
    }
    
    if let Some(non_negative) = non_negative_option {
        if *non_negative {
            let inner = quote! {
                if *val < 0 {
                    return Err(::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be non-negative", stringify!(#field_ident))
                        ),
                        concat!("Use zero or a positive value for '", stringify!(#field_ident), "'.")
                    ));
                }
            };
            let snippet = opt_check!(is_optional, field_ident, #inner);
            checks.push(snippet);
        }
    }
    
    if let Some(min) = min_option {
        let inner = quote! {
            if (*val as i64) < #min {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be at least {}", stringify!(#field_ident), #min)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' is at least ", #min, ".")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(max) = max_option {
        let inner = quote! {
            if (*val as i64) > #max {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be at most {}", stringify!(#field_ident), #max)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' is at most ", #max, ".")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(start) = starts_with_option {
        let inner = quote! {
            if !val.starts_with(#start) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must start with '{}'", stringify!(#field_ident), #start)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' starts with '", #start, "'.")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(end) = ends_with_option {
        let inner = quote! {
            if !val.ends_with(#end) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must end with '{}'", stringify!(#field_ident), #end)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' ends with '", #end, "'.")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(substr) = includes_option {
        let inner = quote! {
            if !val.contains(#substr) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must include '{}'", stringify!(#field_ident), #substr)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' includes '", #substr, "'.")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(true) = alphanumeric_option {
        let inner = quote! {
            if !val.chars().all(|c| c.is_alphanumeric()) {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must contain only alphanumeric characters", stringify!(#field_ident))
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' has only letters and numbers.")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }
    
    if let Some(multiple) = multiple_of_option {
        let inner = quote! {
            if (*val as i64) % #multiple != 0 {
                return Err(::oximod::_attach_printables!(
                    ::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' must be a multiple of {}", stringify!(#field_ident), #multiple)
                    ),
                    concat!("Ensure '", stringify!(#field_ident), "' is divisible by ", #multiple, ".")
                ));
            }
        };
        let snippet = opt_check!(is_optional, field_ident, #inner);
        checks.push(snippet);
    }

    checks
}
