use proc_macro2::TokenStream;
use syn::{LitFloat, LitInt};

/// Represents a numeric literal
#[derive(Debug)]
pub enum LitNum {
    Int { lit: LitInt, neg: bool },
    Float { lit: LitFloat, neg: bool },
}

#[derive(Default, Debug)]
/// Arguments for field validation in OxiMod using the `#[validate(...)]` attribute.
///
/// This struct is populated from the `#[validate(...)]` attribute
/// and specifies the set of validation rules to apply to the field.
///
/// Validation rules are grouped by the type of field they apply to:
///
/// - Length types (String, Vec, HashSet, BTreeSet, arrays, maps, etc.)
/// - String types
/// - Numeric types
/// - Integer-only types
/// - Optional types
/// - Custom validators
///
/// OxiMod performs compile-time checks to ensure incompatible rules
/// are not applied to the wrong field type.
///
/// # Length-type validations
///
/// These rules apply to any type that has a length,
/// such as `String`, `&str`, `Cow<str>`, `Vec<T>`,
/// `HashSet<T>`, `BTreeSet<T>`, `HashMap<K, V>`,
/// `BTreeMap<K, V>`, arrays, etc.
///
/// - `min_length`: (Optional) Minimum allowed length.
///   - The value length must be >= this value.
///   - Default: no minimum-length constraint.
///
/// - `max_length`: (Optional) Maximum allowed length.
///   - The value length must be <= this value.
///   - Default: no maximum-length constraint.
///
/// - `non_empty`: (Optional) Value must not be empty.
///   - Equivalent to `min_length = 1`.
///   - Default: `false`.
///
/// # String validations
///
/// These rules apply only to string-like types.
///
/// - `starts_with`: (Optional) Required prefix.
///   - The string must start with this value.
///
/// - `ends_with`: (Optional) Required suffix.
///   - The string must end with this value.
///
/// - `includes`: (Optional) Required substring.
///   - The string must contain this value.
///
/// - `alphanumeric`: (Optional) Only letters and digits allowed.
///   - Matches `/^[a-zA-Z0-9]+$/`.
///
/// - `email`: (Optional) Must be a valid email format.
///   - Uses a basic email regex.
///
/// - `pattern`: (Optional) Custom regex.
///   - The string must match this pattern.
///
/// # Numeric validations
///
/// These rules apply to numeric types.
///
/// - `min`: (Optional) Minimum allowed value (inclusive).
///   - Value must be >= this.
///
/// - `max`: (Optional) Maximum allowed value (inclusive).
///   - Value must be <= this.
///
/// - `min_exclusive`: (Optional) Minimum bound is exclusive.
///   - When true, `min` becomes strictly greater than.
///
/// - `max_exclusive`: (Optional) Maximum bound is exclusive.
///   - When true, `max` becomes strictly less than.
///
/// - `positive`: (Optional) Value must be > 0.
///
/// - `negative`: (Optional) Value must be < 0.
///
/// - `non_negative`: (Optional) Value must be >= 0.
///
/// - `non_positive`: (Optional) Value must be <= 0.
///
/// # Integer-only validations
///
/// These rules apply only to integer types.
///
/// - `multiple_of`: (Optional) Must be divisible by the given number.
///
/// # Optional validations
///
/// These rules apply to `Option<T>` fields.
///
/// - `required`: (Optional) Value must be present.
///   - `None` will cause validation error.
///
/// # Custom validation
///
/// Allows using a user-defined function.
///
/// The function must have the signature:
///
/// ```text
/// fn(&T) -> Result<(), String>
/// ```
///
/// where `T` is the field type.
///
/// Example:
///
/// ```rust
/// fn validate_name(value: &String) -> Result<(), String> {
///     if value == "admin" {
///         return Err("reserved name".into());
///     }
///     Ok(())
/// }
/// ```
///
/// Usage:
///
/// ```text
/// #[validate(custom(validate_name))]
/// name: String
/// ```
///
/// Custom validators run after built-in validations.
///
/// # Example
///
/// ```ignore
/// #[derive(Model)]
/// struct User {
///     #[validate(
///         required,
///         min_length = 3,
///         max_length = 30,
///         pattern = r"^[a-zA-Z0-9_]+$"
///     )]
///     username: String,
///
///     #[validate(email)]
///     email: Option<String>,
///
///     #[validate(non_negative, max = 100)]
///     score: i64,
///
///     #[validate(min_length = 1)]
///     tags: Vec<String>,
///
///     #[validate(custom(validate_name))]
///     name: String,
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
