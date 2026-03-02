use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn generate_model_token(name: &Ident, db: &str, collection: &str) -> TokenStream {
    quote! {
        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            fn get_collection_with_client(client: &::oximod::_mongodb::Client) -> Result<
                ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>,
                ::oximod::_error::oximod_error::OxiModError
            > {
                let db = client.database(#db);
                Ok(db.collection::<::oximod::_mongodb::bson::Document>(#collection))
            }

            fn get_collection() -> Result<
                ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>,
                ::oximod::_error::oximod_error::OxiModError
            > {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::get_collection_with_client(client)
            }

            async fn save_with_client(&self, client: &::oximod::_mongodb::Client) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OxiModError> {
                self.validate()?;
                let collection = Self::get_collection_with_client(client)?;
                Self::_create_indexes(&collection).await?;

                let document = ::oximod::_mongodb::bson::to_document(&self).map_err(|e|
                    ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}"))
                )?;

                let result = collection.insert_one(document).await.map_err(|e|
                    ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}"))
                )?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err(
                        ::oximod::_error::oximod_error::OxiModError::SerializationError("inserted_id is not an ObjectId".to_string())
                    )
                }
            }

            async fn save(&self) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                self.save_with_client(client).await
            }


            async fn update_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let result = collection
                    .update_many(filter.into(), update.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(result)
            }

            async fn update(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::update_with_client(filter, update, client).await
            }


            async fn update_one_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let result = collection
                    .update_one(filter.into(), update.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(result)
            }

            async fn update_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::update_one_with_client(filter, update, client).await
            }

            async fn delete_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let result = collection
                    .delete_many(filter.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(result)
            }

            async fn delete(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::delete_with_client(filter, client).await
            }

            async fn delete_one_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let result = collection
                    .delete_one(filter.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(result)
            }

            async fn delete_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::delete_one_with_client(filter, client).await
            }

            async fn find_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<Vec<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                let collection = Self::get_collection()?;


                let mut cursor = collection
                    .find(filter.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                let mut results = vec![];

                while let Some(doc) = ::oximod::_futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc.map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                    let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}")))?;

                    results.push(parsed);
                }

                Ok(results)
            }

            async fn find(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<Vec<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::find_with_client(filter, client).await
            }

            async fn find_one_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                let collection = Self::get_collection_with_client(client)?;


                let result = collection
                    .find_one(filter.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                match result {
                    Some(doc) => {
                        let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}")))?;
                        Ok(Some(parsed))
                    }
                    None => Ok(None),
                }
            }

            async fn find_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::find_one_with_client(filter, client).await
            }


            async fn find_by_id_with_client(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                Self::find_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, client).await
            }

            async fn find_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OxiModError>
            where
                Self: Sized,
            {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::find_by_id_with_client(id, client).await
            }

            async fn update_by_id_with_client(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                Self::update_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, update, client).await
            }

            async fn update_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::update_by_id_with_client(id, update, client).await
            }

            async fn delete_by_id_with_client(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                client: &::oximod::_mongodb::Client
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                Self::delete_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, client).await
            }

            async fn delete_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::delete_by_id_with_client(id, client).await
            }

            async fn count_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<u64, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let count = collection
                    .count_documents(filter.into())
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(count)
            }

            async fn count(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<u64, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::count_with_client(filter, client).await
            }

            async fn exists_with_client(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                client: &::oximod::_mongodb::Client
            ) -> Result<bool, ::oximod::_error::oximod_error::OxiModError> {
                Self::find_one_with_client(filter, client).await
                    .map(|opt| opt.is_some())
            }

            async fn exists(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<bool, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::exists_with_client(filter, client).await
            }

            async fn clear_with_client(client: &::oximod::_mongodb::Client) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let collection = Self::get_collection_with_client(client)?;

                let result = collection
                    .delete_many(::oximod::_mongodb::bson::doc! {})
                    .await
                    .map_err(|e| ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")))?;

                Ok(result)
            }

            async fn clear() -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OxiModError> {
                let client_arc = ::oximod::_feature::conn::client::OxiClient::global()?;
                let client: &::oximod::_mongodb::Client = client_arc.as_ref();
                Self::clear_with_client(&client).await
            }
        }
    }
}
