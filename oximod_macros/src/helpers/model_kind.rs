//! Parsing of collection and embedded model modes.
//!
//! Models are collection-backed by default. Applying `#[model(embedded)]`
//! changes the generated model to embedded mode.
//!
//! Only one `model` attribute is accepted, and `embedded` is currently the
//! only supported explicit option.

use proc_macro2::TokenStream;
use syn::{Attribute, Ident};

/// Determines which functionality the `Model` derive generates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// A model backed by its own MongoDB collection.
    Collection,

    /// A model stored inside another document.
    Embedded,
}

impl ModelKind {
    /// Resolves the model kind from derive attributes.
    ///
    /// Models without a `#[model(...)]` attribute default to
    /// [`ModelKind::Collection`].
    ///
    /// # Errors
    ///
    /// Returns compile-error tokens when:
    ///
    /// - more than one `model` attribute is present;
    /// - the attribute argument is malformed;
    /// - the requested model option is unsupported.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self, TokenStream> {
        let mut kind = None;

        for attr in attrs {
            if !attr.path().is_ident("model") {
                continue;
            }

            if kind.is_some() {
                return Err(
                    syn::Error::new_spanned(attr, "duplicate `model` attribute").to_compile_error()
                );
            }

            let option = attr.parse_args::<Ident>().map_err(|error| {
                syn::Error::new_spanned(attr, format!("invalid `model` attribute: {error}"))
                    .to_compile_error()
            })?;

            let parsed_kind = if option == "embedded" {
                Self::Embedded
            } else {
                return Err(syn::Error::new_spanned(
                    option,
                    "unsupported model option; expected `embedded`",
                )
                .to_compile_error());
            };

            kind = Some(parsed_kind);
        }

        Ok(kind.unwrap_or(Self::Collection))
    }
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, parse_quote};

    use super::ModelKind;

    #[test]
    fn models_are_collection_backed_by_default() {
        let attrs = Vec::<Attribute>::new();

        let kind = ModelKind::from_attrs(&attrs).expect("model kind should parse");

        assert_eq!(kind, ModelKind::Collection);
    }

    #[test]
    fn embedded_attribute_selects_embedded_mode() {
        let attrs = vec![parse_quote! {
            #[model(embedded)]
        }];

        let kind = ModelKind::from_attrs(&attrs).expect("embedded model kind should parse");

        assert_eq!(kind, ModelKind::Embedded);
    }

    #[test]
    fn unrelated_attributes_are_ignored() {
        let attrs = vec![
            parse_quote! {
                #[serde(rename_all = "camelCase")]
            },
            parse_quote! {
                #[hooks]
            },
        ];

        let kind = ModelKind::from_attrs(&attrs).expect("unrelated attributes should be ignored");

        assert_eq!(kind, ModelKind::Collection);
    }

    #[test]
    fn duplicate_model_attributes_are_rejected() {
        let attrs = vec![
            parse_quote! {
                #[model(embedded)]
            },
            parse_quote! {
                #[model(embedded)]
            },
        ];

        let error = ModelKind::from_attrs(&attrs)
            .expect_err("duplicate model attributes should fail")
            .to_string();

        assert!(
            error.contains("duplicate `model` attribute"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn unsupported_model_options_are_rejected() {
        let attrs = vec![parse_quote! {
            #[model(collection)]
        }];

        let error = ModelKind::from_attrs(&attrs)
            .expect_err("unsupported model option should fail")
            .to_string();

        assert!(
            error.contains("unsupported model option; expected `embedded`"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn malformed_model_attributes_are_rejected() {
        let attrs = vec![parse_quote! {
            #[model("embedded")]
        }];

        let error = ModelKind::from_attrs(&attrs)
            .expect_err("malformed model attribute should fail")
            .to_string();

        assert!(
            error.contains("invalid `model` attribute"),
            "unexpected error tokens: {error}"
        );
    }
}
