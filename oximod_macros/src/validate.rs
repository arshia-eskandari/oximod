use syn::{ Attribute, Lit };

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
    // pub enum_values: Option<Vec<String>>, // use rust's enum instead
    pub email: Option<bool>,
    pub pattern: Option<String>,
    pub non_empty: Option<bool>,
    pub positive: Option<bool>,
    pub negative: Option<bool>,
    pub non_negative: Option<bool>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

pub struct ValidateDefinition {
    pub field_name: String,
    pub args: ValidateArgs,
}

pub fn parse_validate_args(attr: &Attribute, field_name: String) -> syn::Result<ValidateDefinition> {
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
                // } else if meta.path.is_ident("enum_values") {
                //     // 1. Grab the parenthesized group
                //     let content;
                //     syn::parenthesized!(content in meta.input);

                //     // 2. Parse a comma-separated list of string literals
                //     let values = content
                //         .parse_terminated(
                //             |buf: &syn::parse::ParseBuffer| buf.parse::<syn::LitStr>(), // note the closure
                //             syn::Token![,]
                //         )?
                //         .into_iter()
                //         .map(|lit_str| lit_str.value())
                //         .collect::<Vec<_>>();

                //     args.enum_values = Some(values);
            } else if meta.path.is_ident("email") {
                args.email = Some(true);
            } else if meta.path.is_ident("pattern") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.pattern = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `min_length`")
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
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(ValidateDefinition { field_name, args })
}
