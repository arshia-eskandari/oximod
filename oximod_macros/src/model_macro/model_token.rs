//! Model-trait implementation generation.
//!
//! This module generates:
//!
//! - validation support shared by collection and embedded models;
//! - the public inherent `validate()` method;
//! - MongoDB persistence methods available only to collection models.
//!
//! Hook invocation tokens are generated separately by `hook_tokens`.

use crate::{
    helpers::ModelKind,
    model_macro::{HookTokens, generate_hook_tokens},
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

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
            fn validate(&self) -> Result<(), ::oximod::OxiModError> {
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

pub fn generate_collection_model_token(
    name: &Ident,
    db: &str,
    collection: &str,
    hooks: bool,
) -> TokenStream {
    let HookTokens {
        pre_save,
        post_save,
        pre_save_mut,
        post_save_mut,
        pre_find,
        post_find,
        pre_delete,
        post_delete,
        pre_update,
        post_update,
    } = generate_hook_tokens(hooks);

    // This avoids cloning `update` when hooks are disabled.
    let update_token = if hooks {
        quote! { update.clone() }
    } else {
        quote! { update }
    };

    quote! {
        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {
            fn get_collection_from(
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::Collection<Self>,
                ::oximod::OxiModError,
            > {
                let db = client.database(#db);

                Ok(db.collection::<Self>(#collection))
            }

            async fn save_from(
                &self,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::bson::oid::ObjectId,
                ::oximod::OxiModError,
            > {
                #pre_save

                let id = self
                    .__oximod_insert_with_client(client)
                    .await?;

                #post_save

                Ok(id)
            }

            async fn save_from_mut(
                &mut self,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::bson::oid::ObjectId,
                ::oximod::OxiModError,
            > {
                #pre_save_mut

                let id = self
                    .__oximod_insert_with_client(client)
                    .await?;

                #post_save_mut

                Ok(id)
            }

            async fn find_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                Option<Self>,
                ::oximod::OxiModError,
            > {
                #pre_find

                let collection =
                    Self::get_collection_from(client)?;

                let result = collection
                    .find_one(
                        ::oximod::_mongodb::bson::doc! {
                            "_id": id.clone(),
                        },
                    )
                    .await
                    .map_err(|error| {
                        ::oximod::OxiModError::database(
                            "Failed to find document by _id",
                            error,
                        )
                    })?;

                #post_find

                Ok(result)
            }

            async fn delete_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::results::DeleteResult,
                ::oximod::OxiModError,
            > {
                #pre_delete

                let collection =
                    Self::get_collection_from(client)?;

                let result = collection
                    .delete_one(
                        ::oximod::_mongodb::bson::doc! {
                            "_id": id.clone(),
                        },
                    )
                    .await
                    .map_err(|error| {
                        ::oximod::OxiModError::database(
                            "Failed to delete document by _id",
                            error,
                        )
                    })?;

                #post_delete

                Ok(result)
            }

            async fn update_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: ::oximod::_mongodb::bson::Document,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::results::UpdateResult,
                ::oximod::OxiModError,
            > {
                #pre_update

                let collection =
                    Self::get_collection_from(client)?;

                let result = collection
                    .update_one(
                        ::oximod::_mongodb::bson::doc! {
                            "_id": id.clone(),
                        },
                        #update_token,
                    )
                    .await
                    .map_err(|error| {
                        ::oximod::OxiModError::database(
                            "Failed to update document by _id",
                            error,
                        )
                    })?;

                #post_update

                Ok(result)
            }

            async fn clear_from(
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::results::DeleteResult,
                ::oximod::OxiModError,
            > {
                let collection =
                    Self::get_collection_from(client)?;

                let result = collection
                    .delete_many(
                        ::oximod::_mongodb::bson::doc! {},
                    )
                    .await
                    .map_err(|error| {
                        ::oximod::OxiModError::database(
                            "Failed to execute MongoDB delete_many operation",
                            error,
                        )
                    })?;

                Ok(result)
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
