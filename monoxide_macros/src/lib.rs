use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, DeriveInput, LitStr };

#[proc_macro_derive(Model, attributes(db, collection))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut db: Option<LitStr> = None;
    let mut collection: Option<LitStr> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("db") {
            if let Ok(val) = attr.parse_args::<LitStr>() {
                db = Some(val);
            } else {
                return syn::Error
                    ::new_spanned(attr, "Expected #[db(\"db_name\"]")
                    .to_compile_error()
                    .into();
            }
        } else if attr.path().is_ident("collection") {
            if let Ok(val) = attr.parse_args::<LitStr>() {
                collection = Some(val);
            } else {
                return syn::Error
                    ::new_spanned(attr, "Expected #[collection(\"collection_name\"]")
                    .to_compile_error()
                    .into();
            }
        }
    }

    let db = match db {
        Some(val) => val,
        None => {
            return syn::Error
                ::new_spanned(&input, "Missing #[db(\"db_name\"] attribute")
                .to_compile_error()
                .into();
        }
    };

    let collection = match collection {
        Some(val) => val,
        None => {
            return syn::Error
                ::new_spanned(&input, "Missing #[collection(\"collection_name\"] attribute")
                .to_compile_error()
                .into();
        }
    };

    let expanded =
        quote! {
        use ::monoxide_core::error::printable::Printable;

        fn get_collection() -> Result<
            ::mongodb::Collection<::mongodb::bson::Document>, 
            ::monoxide_core::error::conn_error::MongoDbError
        > {
            let client = ::monoxide_core::feature::conn::client::get_global_client()?;
            let db = client.database(#db);
            Ok(db.collection::<::mongodb::bson::Document>(#collection))
        }

        #[async_trait::async_trait]
        impl ::monoxide_core::feature::model::Model for #name {

            async fn save(&self) -> Result<::mongodb::bson::oid::ObjectId, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let document = ::mongodb::bson::to_document(&self).map_err(|e| {
                    ::monoxide_core::attach_printables!(
                        ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()),
                        "Failed to serialize model. Are all field types supported by bson::to_document()?"
                    )
                })?;

                let result = collection.insert_one(document).await.map_err(|e| {
                    ::monoxide_core::attach_printables!(
                        ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                        "Failed to insert document. Check if the MongoDB server is reachable and the collection exists."
                    )
                })?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err( ::monoxide_core::attach_printables!(
                        ::monoxide_core::error::conn_error::MongoDbError::SerializationError("inserted_id is not an ObjectId".to_string()),
                        "Expected inserted_id to be an ObjectId but received something else. This may happen if you're using a custom _id."
                    ))
                }
            }

            async fn update(
                filter: impl Into<::mongodb::bson::Document> + Send,
                update: impl Into<::mongodb::bson::Document> + Send
            ) -> Result<::mongodb::results::UpdateResult, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let result = collection
                    .update_many(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to update documents. Check your update operators and filter structure."
                        )
                    })?;

                Ok(result)
            }

            async fn update_one(
                filter: impl Into<::mongodb::bson::Document> + Send,
                update: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<::mongodb::results::UpdateResult, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let result = collection
                    .update_one(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to update a document. Make sure your update syntax is valid and the filter matches at least one document."
                        )
                    })?;

                Ok(result)
            }

            async fn delete(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<::mongodb::results::DeleteResult, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let result = collection
                    .delete_many(filter.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to delete documents. Ensure your filter is valid and matches the correct documents."
                        )
                    })?;

                Ok(result)
            }

            async fn delete_one(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<::mongodb::results::DeleteResult, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let result = collection
                    .delete_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to delete a single document. Ensure your filter is valid and matches the correct document."
                        )
                    })?;

                Ok(result)
            }

            async fn find(
                filter: impl Into<::mongodb::bson::Document> + Send
            ) -> Result<Vec<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where
                Self: Sized,
            {
                let collection = get_collection()?;

                let mut cursor = collection
                    .find(filter.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to execute find query. Double-check your filter syntax or collection state."
                        )
                    })?;

                let mut results = vec![];

                while let Some(doc) = ::futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc.map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Cursor failed to retrieve a document. This may indicate a deserialization or network error mid-stream."
                        )
                    })?;

                    let parsed = ::mongodb::bson::from_document(doc).map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()),
                            "Failed to deserialize document into model. Check field types and optionality."
                        )
                    })?;

                    results.push(parsed);
                }

                Ok(results)
            }

            async fn find_one(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<Option<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where
                Self: Sized,
            {
                let collection = get_collection()?;

                let result = collection
                    .find_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to run find_one query. Ensure your filter is structured properly and the collection is accessible."
                        )
                    })?;

                match result {
                    Some(doc) => {
                        let parsed = ::mongodb::bson::from_document(doc).map_err(|e| {
                            ::monoxide_core::attach_printables!(
                                ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()),
                                "Could not deserialize document into model. Check for type mismatches or missing #[serde] attributes."
                            )
                        })?;
                        Ok(Some(parsed))
                    }
                    None => Ok(None),
                }
            }

            async fn find_by_id(
                id: ::mongodb::bson::oid::ObjectId,
            ) -> Result<Option<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where
                Self: Sized,
            {
                Self::find_one(::mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::monoxide_core::attach_printables!(
                        e,
                        "Failed to find document by _id. Confirm the ID is valid and the document exists."
                    )
                })
            }

            async fn update_by_id(
                id: ::mongodb::bson::oid::ObjectId,
                update: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<::mongodb::results::UpdateResult, ::monoxide_core::error::conn_error::MongoDbError> {
                Self::update_one(::mongodb::bson::doc! { "_id": id }, update).await.map_err(|e| {
                    ::monoxide_core::attach_printables!(
                        e,
                        "Failed to update document by _id. Check if the document exists and if your update operators are valid."
                    )
                })
            }

            async fn delete_by_id(
                id: ::mongodb::bson::oid::ObjectId,
            ) -> Result<::mongodb::results::DeleteResult, ::monoxide_core::error::conn_error::MongoDbError> {
                Self::delete_one(::mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::monoxide_core::attach_printables!(
                        e,
                        "Failed to delete document by _id. Ensure the ID is correct and that the document exists."
                    )
                })
            }

            async fn count(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<u64, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let count = collection
                    .count_documents(filter.into())
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to count documents. Make sure the filter is well-formed and the collection is accessible."
                        )
                    })?;

                Ok(count)
            }

            async fn exists(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<bool, ::monoxide_core::error::conn_error::MongoDbError> {
                Self::find_one(filter).await
                    .map(|opt| opt.is_some())
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            e,
                            "Failed to check document existence. Make sure your filter is valid and your connection is healthy."
                        )
                    })
            }

            async fn clear() -> Result<::mongodb::results::DeleteResult, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;

                let result = collection
                    .delete_many(::mongodb::bson::doc! {})
                    .await
                    .map_err(|e| {
                        ::monoxide_core::attach_printables!(
                            ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()),
                            "Failed to clear the collection. Ensure the MongoDB connection is valid and the collection is writable."
                        )
                    })?;

                Ok(result)
            }
        }
    };

    expanded.into()
}
