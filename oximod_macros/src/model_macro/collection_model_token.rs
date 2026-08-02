//! Collection-model persistence token generation.
//!
//! This module generates the public `Model` implementation for models backed
//! by MongoDB collections. It includes collection access and the generated
//! save, find, update, delete, and clear operations.
//!
//! Embedded models do not receive this implementation.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use super::hook_tokens::{HookTokens, generate_hook_tokens};

/// Generates the collection-backed `Model` implementation.
///
/// Hook invocations are included only when `hooks` is enabled.
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

    // Avoid cloning the update document when hooks are disabled.
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
