//! Validation argument types and rule classification.
//!
//! `ValidateArgs` stores the rules parsed from a field's
//! `#[validate(...)]` attribute. Its classification methods determine which
//! field-type checks and runtime validation tokens must be generated.
//!
//! This module also contains:
//!
//! - `LitNum`, which preserves numeric literal syntax and sign;
//! - `PrimitiveNum`, which classifies supported primitive numeric types;
//! - `BuiltChecks`, which accumulates generated validation token streams.

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
///   - Every character must be an ASCII letter or digit; an empty string
///     passes (combine with `non_empty` to reject it).
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
/// # Nested validation
///
/// Applies to embedded OxiMod models and supported containers of them.
///
/// - `nested`: (Optional) Recursively validate the embedded value.
///   - Valid on a field whose type is an embedded OxiMod model or a
///     supported container (`Option`, `Vec`, `HashMap<String, _>`, and
///     recursive combinations) of one.
///   - Descendant validation errors are reported with path-aware field
///     names such as `address.postal_code` or `items[1].sku`.
///   - Default: no descent into embedded models.
///
/// # Custom validation
///
/// Allows using a user-defined function.
///
/// The function must have the signature:
///
/// ```text
/// fn(&T) -> Result<(), E>
/// ```
///
/// where `T` is the field type and `E` implements `ToString`. For an
/// `Option<T>` field, the validator receives `&T` and runs only when the
/// field contains `Some(value)`.
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
/// Custom validators run in addition to built-in validations; all resulting
/// errors are aggregated into one validation failure.
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

    // must be an embedded OxiMod model or a supported container of one
    pub nested: bool,

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

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::{LitNum, ValidateArgs};

    #[test]
    fn default_arguments_require_no_specific_field_type() {
        let args = ValidateArgs::default();

        assert!(!args.must_be_length_type());
        assert!(!args.must_be_string());
        assert!(!args.must_be_signed_number());
        assert!(!args.must_be_integer());
        assert!(!args.must_be_number());
        assert!(!args.must_be_optional());
        assert!(!args.has_type_collision());
    }

    #[test]
    fn length_rules_require_length_capable_fields() {
        let min_length = ValidateArgs {
            min_length: Some(1),
            ..Default::default()
        };

        assert!(min_length.must_be_length_type());

        let max_length = ValidateArgs {
            max_length: Some(10),
            ..Default::default()
        };

        assert!(max_length.must_be_length_type());

        let non_empty = ValidateArgs {
            non_empty: true,
            ..Default::default()
        };

        assert!(non_empty.must_be_length_type());
    }

    #[test]
    fn string_rules_require_string_fields() {
        let rules = [
            ValidateArgs {
                starts_with: Some("User".into()),
                ..Default::default()
            },
            ValidateArgs {
                ends_with: Some("1".into()),
                ..Default::default()
            },
            ValidateArgs {
                includes: Some("ser".into()),
                ..Default::default()
            },
            ValidateArgs {
                alphanumeric: true,
                ..Default::default()
            },
            ValidateArgs {
                email: true,
                ..Default::default()
            },
            ValidateArgs {
                pattern: Some("^[a-z]+$".into()),
                ..Default::default()
            },
        ];

        for args in rules {
            assert!(
                args.must_be_string(),
                "every string rule should require a string field"
            );
        }
    }

    #[test]
    fn sign_rules_require_signed_numeric_fields() {
        let rules = [
            ValidateArgs {
                positive: true,
                ..Default::default()
            },
            ValidateArgs {
                negative: true,
                ..Default::default()
            },
            ValidateArgs {
                non_negative: true,
                ..Default::default()
            },
            ValidateArgs {
                non_positive: true,
                ..Default::default()
            },
        ];

        for args in rules {
            assert!(args.must_be_signed_number());
            assert!(args.must_be_number());
        }
    }

    #[test]
    fn numeric_bounds_require_numeric_fields() {
        let minimum = ValidateArgs {
            min: Some(LitNum::Int {
                lit: parse_quote!(10),
                neg: false,
            }),
            ..Default::default()
        };

        assert!(minimum.must_be_number());
        assert!(!minimum.must_be_integer());

        let maximum = ValidateArgs {
            max: Some(LitNum::Float {
                lit: parse_quote!(10.5),
                neg: false,
            }),
            ..Default::default()
        };

        assert!(maximum.must_be_number());
        assert!(!maximum.must_be_integer());

        let exclusive_minimum = ValidateArgs {
            min_exclusive: true,
            ..Default::default()
        };

        assert!(exclusive_minimum.must_be_number());

        let exclusive_maximum = ValidateArgs {
            max_exclusive: true,
            ..Default::default()
        };

        assert!(exclusive_maximum.must_be_number());
    }

    #[test]
    fn multiple_of_requires_an_integer_field() {
        let args = ValidateArgs {
            multiple_of: Some(parse_quote!(5)),
            ..Default::default()
        };

        assert!(args.must_be_integer());
        assert!(args.must_be_number());
        assert!(!args.must_be_signed_number());
    }

    #[test]
    fn required_rule_requires_an_optional_field() {
        let args = ValidateArgs {
            required: true,
            ..Default::default()
        };

        assert!(args.must_be_optional());
        assert!(!args.must_be_number());
        assert!(!args.must_be_string());
    }

    #[test]
    fn string_and_numeric_rules_have_a_type_collision() {
        let args = ValidateArgs {
            email: true,
            min: Some(LitNum::Int {
                lit: parse_quote!(1),
                neg: false,
            }),
            ..Default::default()
        };

        assert!(args.has_type_collision());
    }

    #[test]
    fn length_and_numeric_rules_have_a_type_collision() {
        let args = ValidateArgs {
            min_length: Some(1),
            max: Some(LitNum::Int {
                lit: parse_quote!(10),
                neg: false,
            }),
            ..Default::default()
        };

        assert!(args.has_type_collision());
    }

    #[test]
    fn length_and_string_rules_can_be_combined() {
        let args = ValidateArgs {
            min_length: Some(3),
            alphanumeric: true,
            ..Default::default()
        };

        assert!(args.must_be_length_type());
        assert!(args.must_be_string());
        assert!(!args.has_type_collision());
    }

    #[test]
    fn custom_validator_does_not_impose_a_builtin_type_group() {
        let args = ValidateArgs {
            custom: Some(parse_quote!(validators::validate_value)),
            ..Default::default()
        };

        assert!(!args.must_be_length_type());
        assert!(!args.must_be_string());
        assert!(!args.must_be_number());
        assert!(!args.must_be_optional());
        assert!(!args.has_type_collision());
    }
}
