use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn generate_model_token(name: &Ident, db: &str, collection: &str) -> TokenStream {
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
                use ::oximod::Hooks;
                self.pre_save().await?;
                let id = self.__oximod_insert_with_client(client).await?;
                self.post_save().await?;
                Ok(id)
            }

            async fn save_from_mut(&mut self, client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::bson::oid::ObjectId,
                ::oximod::_error::oximod_error::OxiModError
            > {
                use ::oximod::Hooks;
                self.pre_save_mut().await?;
                let id = self.__oximod_insert_with_client(client).await?;
                self.post_save_mut().await?;
                Ok(id)
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
