//! Validation token generation for derived models.
//!
//! This module generates validation support shared by collection and embedded
//! models:
//!
//! - the internal `ModelCore` implementation;
//! - aggregation of all configured field-validation errors;
//! - the public inherent `validate()` method.
//!
//! Collection persistence methods are generated separately by
//! `collection_model_token`.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::helpers::ModelKind;

/// Generates validation support for a collection or embedded model.
pub fn generate_model_token(
    name: &Ident,
    kind: ModelKind,
    validations: Vec<TokenStream>,
) -> TokenStream {
    let mode = match kind {
        ModelKind::Collection => {
            quote! {
                ::oximod::_feature::model::Collection
            }
        }

        ModelKind::Embedded => {
            quote! {
                ::oximod::_feature::model::Embedded
            }
        }
    };

    quote! {
        impl ::oximod::_feature::model::ModelCore<#mode> for #name {
            #[inline]
            fn validate(
                &self,
            ) -> Result<(), ::oximod::OxiModError> {
                let mut validation_errors = Vec::new();

                #(#validations)*

                if validation_errors.is_empty() {
                    Ok(())
                } else {
                    Err(
                        ::oximod::OxiModError::validations(
                            validation_errors,
                        ),
                    )
                }
            }
        }

        impl #name {
            /// Validates this model using its configured field validations.
            ///
            /// This inherent method is generated for both collection-backed
            /// and embedded models, so no internal validation trait needs to
            /// be imported by application code.
            #[inline]
            pub fn validate(
                &self,
            ) -> Result<(), ::oximod::OxiModError> {
                <
                    Self as ::oximod::_feature::model::ModelCore<#mode>
                >::validate(self)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};

    use crate::helpers::ModelKind;

    use super::generate_model_token;

    #[test]
    fn collection_model_uses_collection_validation_mode() {
        let name = format_ident!("User");

        let generated = compact(generate_model_token(
            &name,
            ModelKind::Collection,
            Vec::new(),
        ));

        assert!(
            generated.contains("ModelCore<::oximod::_feature::model::Collection>forUser"),
            "collection models should use Collection validation mode; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn embedded_model_uses_embedded_validation_mode() {
        let name = format_ident!("Address");

        let generated = compact(generate_model_token(&name, ModelKind::Embedded, Vec::new()));

        assert!(
            generated.contains("ModelCore<::oximod::_feature::model::Embedded>forAddress"),
            "embedded models should use Embedded validation mode; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn generated_inherent_validate_delegates_to_model_core() {
        let name = format_ident!("User");

        let generated = compact(generate_model_token(
            &name,
            ModelKind::Collection,
            Vec::new(),
        ));

        assert!(
            generated.contains(
                "<Selfas::oximod::_feature::model::ModelCore<\
::oximod::_feature::model::Collection>>::validate(self)"
            ),
            "the inherent validate method should delegate to ModelCore; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn generated_validation_includes_all_field_validations() {
        let name = format_ident!("User");

        let validations = vec![
            quote! {
                validation_errors.push(first_error);
            },
            quote! {
                validation_errors.push(second_error);
            },
        ];

        let generated = compact(generate_model_token(
            &name,
            ModelKind::Collection,
            validations,
        ));

        let first = generated
            .find("validation_errors.push(first_error);")
            .expect("first validation should be generated");

        let second = generated
            .find("validation_errors.push(second_error);")
            .expect("second validation should be generated");

        assert!(
            first < second,
            "field validations should preserve their source order"
        );

        assert!(
            generated.contains(
                "ifvalidation_errors.is_empty(){Ok(())}else{\
Err(::oximod::OxiModError::validations(validation_errors,),)}"
            ),
            "generated validation should aggregate all errors; \
             generated tokens: {generated}"
        );
    }

    fn compact(tokens: TokenStream) -> String {
        tokens.to_string().replace(' ', "")
    }
}
