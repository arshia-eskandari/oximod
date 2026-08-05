//! Internal token-generation macros for model validation.
//!
//! `opt_check!` wraps generated field-validation rules so they work with both
//! ordinary fields and `Option<T>` fields:
//!
//! - optional fields are validated only when they contain `Some(value)`;
//! - ordinary fields are always validated through a borrowed `val` binding.

/// Generates a validation block for an optional or ordinary field.
///
/// When `$is_optional` is true, the generated checks run only for `Some`.
/// Otherwise, `val` borrows the field directly.
macro_rules! opt_check {
    (
        $is_optional:expr,
        $field_ident:expr,
        $($body:tt)*
    ) => {{
        let __field_ident = $field_ident;

        if $is_optional {
            ::quote::quote! {
                if let Some(val) = &self.#__field_ident {
                    $($body)*
                }
            }
        } else {
            ::quote::quote! {
                {
                    let val = &self.#__field_ident;
                    $($body)*
                }
            }
        }
    }};
}

pub(crate) use opt_check;

#[cfg(test)]
mod tests {
    use quote::{ToTokens, format_ident};

    #[test]
    fn optional_fields_are_checked_only_when_present() {
        let field_ident = format_ident!("nickname");

        let generated = opt_check!(
            true,
            &field_ident,
            validate_value(val);
        );

        assert_eq!(
            compact(&generated),
            "ifletSome(val)=&self.nickname{validate_value(val);}"
        );
    }

    #[test]
    fn ordinary_fields_are_checked_directly() {
        let field_ident = format_ident!("name");

        let generated = opt_check!(
            false,
            &field_ident,
            validate_value(val);
        );

        assert_eq!(
            compact(&generated),
            "{letval=&self.name;validate_value(val);}"
        );
    }

    #[test]
    fn generated_blocks_preserve_multiple_rules() {
        let field_ident = format_ident!("age");

        let generated = opt_check!(
            false,
            &field_ident,
            check_minimum(val);
            check_maximum(val);
        );

        let generated = compact(&generated);

        let minimum = generated
            .find("check_minimum(val);")
            .expect("minimum check should be generated");

        let maximum = generated
            .find("check_maximum(val);")
            .expect("maximum check should be generated");

        assert!(
            minimum < maximum,
            "validation rules should preserve their source order"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
