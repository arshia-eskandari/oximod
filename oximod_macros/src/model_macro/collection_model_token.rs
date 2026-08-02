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

#[cfg(test)]
mod tests {
    use quote::{ToTokens, format_ident};
    use syn::{ImplItem, Item, parse2};

    use super::generate_collection_model_token;

    #[test]
    fn generates_collection_model_implementation() {
        let name = format_ident!("User");

        let tokens = generate_collection_model_token(&name, "app_db", "users", false);

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        assert_eq!(generated_file.items.len(), 1);

        let Item::Impl(model_impl) = &generated_file.items[0] else {
            panic!("generated item should be an impl");
        };

        let Some((_, trait_path, _)) = &model_impl.trait_ else {
            panic!("generated impl should target a trait");
        };

        assert_eq!(compact(trait_path), "::oximod::_feature::model::Model");

        assert_eq!(compact(model_impl.self_ty.as_ref()), "User");
    }

    #[test]
    fn generates_all_collection_persistence_methods() {
        let name = format_ident!("User");

        let tokens = generate_collection_model_token(&name, "app_db", "users", false);

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        let Item::Impl(model_impl) = &generated_file.items[0] else {
            panic!("generated item should be an impl");
        };

        let method_names = model_impl
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(function) => Some(function.sig.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            method_names,
            [
                "get_collection_from",
                "save_from",
                "save_from_mut",
                "find_by_id_from",
                "delete_by_id_from",
                "update_by_id_from",
                "clear_from",
            ]
        );
    }

    #[test]
    fn uses_configured_database_and_collection_names() {
        let name = format_ident!("User");

        let generated = compact(&generate_collection_model_token(
            &name, "app_db", "users", false,
        ));

        assert!(
            generated.contains("client.database(\"app_db\")"),
            "generated implementation should use the configured database; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("db.collection::<Self>(\"users\")"),
            "generated implementation should use the configured collection; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn disabled_hooks_generate_no_hook_calls_or_update_clone() {
        let name = format_ident!("User");

        let generated = compact(&generate_collection_model_token(
            &name, "app_db", "users", false,
        ));

        assert!(
            !generated.contains("::oximod::Hooks"),
            "disabled hooks should not add hook calls; \
             generated tokens: {generated}"
        );

        assert!(
            !generated.contains("update.clone()"),
            "updates should not be cloned when hooks are disabled; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn enabled_hooks_generate_hook_calls_and_update_clone() {
        let name = format_ident!("User");

        let generated = compact(&generate_collection_model_token(
            &name, "app_db", "users", true,
        ));

        assert!(
            generated.contains("<Selfas::oximod::Hooks>::pre_save(self).await?;"),
            "enabled hooks should generate pre-save invocation; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("<Selfas::oximod::Hooks>::post_update(id,&update,).await?;"),
            "enabled hooks should generate post-update invocation; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("update.clone()"),
            "the update document should be cloned when hooks need it \
             after the database operation; generated tokens: {generated}"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
