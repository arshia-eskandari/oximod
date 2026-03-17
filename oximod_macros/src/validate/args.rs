use proc_macro2::TokenStream;
use syn::{LitFloat, LitInt};

/// Represents a numeric literal
#[derive(Debug)]
pub enum LitNum {
    Int { lit: LitInt, neg: bool },
    Float { lit: LitFloat, neg: bool },
}

/// TODO: add docs for new fields
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
    // must be length type
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub non_empty: bool,

    // must be string
    pub starts_with: Option<String>,
    pub ends_with: Option<String>,
    pub includes: Option<String>,
    pub alphanumeric: bool,
    pub email: bool,
    pub pattern: Option<String>,

    // must be signed number
    pub positive: bool,
    pub negative: bool,
    pub non_negative: bool,
    pub non_positive: bool,

    // must be number
    pub min: Option<LitNum>,
    pub max: Option<LitNum>,
    pub min_exclusive: bool,
    pub max_exclusive: bool,

    // must be integer
    pub multiple_of: Option<syn::LitInt>,

    // must be optional
    pub required: bool,

    pub custom: Option<syn::Path>,
}

impl ValidateArgs {
    pub fn must_be_length_type(&self) -> bool {
        self.min_length.is_some() || self.max_length.is_some() || self.non_empty
    }

    pub fn must_be_string(&self) -> bool {
        self.starts_with.is_some()
            || self.ends_with.is_some()
            || self.includes.is_some()
            || self.alphanumeric
            || self.email
            || self.pattern.is_some()
    }

    pub fn must_be_signed_number(&self) -> bool {
        self.positive || self.negative || self.non_negative || self.non_positive
    }

    pub fn must_be_integer(&self) -> bool {
        self.multiple_of.is_some()
    }

    pub fn must_be_number(&self) -> bool {
        self.min.is_some()
            || self.max.is_some()
            || self.multiple_of.is_some()
            || self.positive
            || self.negative
            || self.non_negative
            || self.non_positive
            || self.min_exclusive
            || self.max_exclusive
    }

    pub fn must_be_optional(&self) -> bool {
        self.required
    }

    pub fn has_type_collision(&self) -> bool {
        (self.must_be_length_type() || self.must_be_string()) && self.must_be_number()
    }
}

/// Includes all numeric types
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrimitiveNum {
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

/// Includes all validation token streams
#[derive(Default)]
pub struct BuiltChecks {
    pub checks: Vec<TokenStream>,
    pub field_rules_val: Vec<TokenStream>,
    pub compile_errors: Vec<TokenStream>,
    pub field_rules_direct: Vec<TokenStream>,
    pub numeric_checks: Vec<TokenStream>,
}
