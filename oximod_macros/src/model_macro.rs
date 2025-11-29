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

                let document = ::oximod::_mongodb::bson::to_document(&self).map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}")),
                        @static "Failed to serialize model. Are all field types supported by bson::to_document()?"
                    )
                })?;

                let result = collection.insert_one(document).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                        @static "Failed to insert document. Check if the mongodb server is reachable and the collection exists."
                    )
                })?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err( ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OxiModError::SerializationError("inserted_id is not an ObjectId".to_string()),
                        @static "Expected inserted_id to be an ObjectId but received something else. This may happen if you're using a custom _id."
                    ))
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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to update documents. Check your update operators and filter structure."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to update a document. Make sure your update syntax is valid and the filter matches at least one document."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to delete documents. Ensure your filter is valid and matches the correct documents."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to delete a single document. Ensure your filter is valid and matches the correct document."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to execute find query. Double-check your filter syntax or collection state."
                        )
                    })?;

                let mut results = vec![];

                while let Some(doc) = ::oximod::_futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc.map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Cursor failed to retrieve a document. This may indicate a deserialization or network error mid-stream."
                        )
                    })?;

                    let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}")),
                            @static "Failed to deserialize document into model. Check field types and optionality."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to run find_one query. Ensure your filter is structured properly and the collection is accessible."
                        )
                    })?;

                match result {
                    Some(doc) => {
                        let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                            ::oximod::_attach_printables!(
                                ::oximod::_error::oximod_error::OxiModError::SerializationError(::std::format!("{e}")),
                                @static "Could not deserialize document into model. Check for type mismatches or missing #[serde] attributes."
                            )
                        })?;
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
                Self::find_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, client).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        @static "Failed to find document by _id. Confirm the ID is valid and the document exists."
                    )
                })
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
                Self::update_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, update, client).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        @static "Failed to update document by _id. Check if the document exists and if your update operators are valid."
                    )
                })
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
                Self::delete_one_with_client(::oximod::_mongodb::bson::doc! { "_id": id }, client).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        @static "Failed to delete document by _id. Ensure the ID is correct and that the document exists."
                    )
                })
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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to count documents. Make sure the filter is well-formed and the collection is accessible."
                        )
                    })?;

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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            e,
                            @static "Failed to check document existence. Make sure your filter is valid and your connection is healthy."
                        )
                    })
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
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OxiModError::ConnectionError(::std::format!("{e}")),
                            @static "Failed to clear the collection. Ensure the mongodb connection is valid and the collection is writable."
                        )
                    })?;

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
