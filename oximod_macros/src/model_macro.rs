use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub struct HookTokens {
    pub pre_save: TokenStream,
    pub post_save: TokenStream,
    pub pre_save_mut: TokenStream,
    pub post_save_mut: TokenStream,
    pub pre_find: TokenStream,
    pub post_find: TokenStream,
    pub pre_delete: TokenStream,
    pub post_delete: TokenStream,
    pub pre_update: TokenStream,
    pub post_update: TokenStream,
}

pub fn generate_hook_tokens(hooks: bool) -> HookTokens {
    HookTokens {
        pre_save: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_save(self).await?; }
        } else {
            quote! {}
        },

        post_save: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_save(self).await?; }
        } else {
            quote! {}
        },

        pre_save_mut: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_save_mut(self).await?; }
        } else {
            quote! {}
        },

        post_save_mut: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_save_mut(self).await?; }
        } else {
            quote! {}
        },

        pre_find: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_find(id.clone()).await?; }
        } else {
            quote! {}
        },

        post_find: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_find(&result).await?; }
        } else {
            quote! {}
        },

        pre_delete: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_delete(id.clone()).await?; }
        } else {
            quote! {}
        },

        post_delete: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_delete(id).await?; }
        } else {
            quote! {}
        },

        pre_update: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_update(id.clone(), &update).await?; }
        } else {
            quote! {}
        },

        post_update: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_update(id, &update).await?; }
        } else {
            quote! {}
        },
    }
}

pub fn generate_model_token(name: &Ident, db: &str, collection: &str, hooks: bool) -> TokenStream {
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

    quote! {
        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            fn get_collection_from(client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::Collection<Self>,
                ::oximod::_error::oximod_error::OxiModError
            > {
                let db = client.database(#db);
                Ok(db.collection::<Self>(#collection))
            }

            async fn save_from(&self, client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::bson::oid::ObjectId,
                ::oximod::_error::oximod_error::OxiModError
            > {
                #pre_save
                let id = self.__oximod_insert_with_client(client).await?;
                #post_save
                Ok(id)
            }

            async fn save_from_mut(&mut self, client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::bson::oid::ObjectId,
                ::oximod::_error::oximod_error::OxiModError
            > {
                #pre_save_mut
                let id = self.__oximod_insert_with_client(client).await?;
                #post_save_mut
                Ok(id)
            }

            async fn find_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                Option<Self>,
                ::oximod::_error::oximod_error::OxiModError
            > {
                #pre_find
                let collection = Self::get_collection_from(client)?;
                let result = collection
                    .find_one(::oximod::_mongodb::bson::doc! { "_id": id })
                    .await
                    .map_err(|e|
                        ::oximod::_error::oximod_error::OxiModError::database("Failed to find document by _id", e)
                    )?;
                #post_find
                Ok(result)
            }

            async fn delete_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::results::DeleteResult,
                ::oximod::_error::oximod_error::OxiModError
            > {
                #pre_delete
                let collection = Self::get_collection_from(client)?;
                let result = collection
                    .delete_one(::oximod::_mongodb::bson::doc! { "_id": id })
                    .await
                    .map_err(|e|
                        ::oximod::_error::oximod_error::OxiModError::database("Failed to delete document by _id", e)
                    )?;
                #post_delete
                Ok(result)
            }

            async fn update_by_id_from(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: ::oximod::_mongodb::bson::Document,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                ::oximod::_mongodb::results::UpdateResult,
                ::oximod::_error::oximod_error::OxiModError
            > {
                #pre_update
                let collection = Self::get_collection_from(client)?;
                let result = collection
                    .update_one(::oximod::_mongodb::bson::doc! { "_id": id }, update)
                    .await
                    .map_err(|e|
                        ::oximod::_error::oximod_error::OxiModError::database("Failed to update document by _id", e)
                    )?;
                #post_update
                Ok(result)
            }

            async fn clear_from(client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::results::DeleteResult,
                ::oximod::_error::oximod_error::OxiModError
            > {
                let collection = Self::get_collection_from(client)?;

                let result = collection
                    .delete_many(::oximod::_mongodb::bson::doc! {})
                    .await
                    .map_err(|e|
                        ::oximod::_error::oximod_error::OxiModError::database(
                            "Failed to execute MongoDB delete_many operation",
                            e,
                        )
                )?;

                Ok(result)
            }
        }
    }
}
