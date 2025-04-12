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

            async fn save(&self) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                let document = ::mongodb::bson::to_document(&self).map_err(|e| 
                    ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string())
                )?;
                collection.insert_one(document).await.map_err(|e| {
                    ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(format!("{}", e))
                })?;
                Ok(())
            }

            async fn update(
                filter: impl Into<::mongodb::bson::Document> + Send,
                update: impl Into<::mongodb::bson::Document> + Send
            ) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                collection
                    .update_many(filter.into(), update.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;

                Ok(())
            }

            async fn update_one (
                filter: impl Into<::mongodb::bson::Document> + Send,
                update: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                collection
                    .update_one(filter.into(), update.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;

                Ok(())
            }

            async fn delete(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<u64, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                let result = collection.delete_many(filter.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;
                Ok(result.deleted_count)
            }

            async fn delete_one(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<bool, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                let result = collection.delete_one(filter.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;
                Ok(result.deleted_count > 0)
            }

            async fn find(
                filter: impl Into<::mongodb::bson::Document> + Send
            ) -> Result<Vec<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where Self: Sized {
                let collection = get_collection()?;
                
                let mut cursor = collection
                    .find(filter.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;

                let mut results = vec![];
                while let Some(doc) = ::futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc
                        .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;
                    let parsed = ::mongodb::bson::from_document(doc)
                        .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()))?;
                    results.push(parsed);
                }

                Ok(results)
            }

            async fn find_one(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<Option<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where Self: Sized {
                let collection = get_collection()?;
                let result = collection.find_one(filter.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;

                match result {
                    Some(doc) => {
                        let parsed = ::mongodb::bson::from_document(doc)
                            .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()))?;
                        Ok(Some(parsed))
                    }
                    None => Ok(None),
                }
            }

            async fn find_by_id(
                id: ::mongodb::bson::oid::ObjectId,
            ) -> Result<Option<Self>, ::monoxide_core::error::conn_error::MongoDbError>
            where Self: Sized {
                Self::find_one(::mongodb::bson::doc! { "_id": id }).await
            }

            async fn update_by_id(
                id: ::mongodb::bson::oid::ObjectId,
                update: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                Self::update_one(::mongodb::bson::doc! { "_id": id }, update).await
            }

            async fn delete_by_id(
                id: ::mongodb::bson::oid::ObjectId,
            ) -> Result<bool, ::monoxide_core::error::conn_error::MongoDbError> {
                Self::delete_one(::mongodb::bson::doc! { "_id": id }).await
            }

            async fn count(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<u64, ::monoxide_core::error::conn_error::MongoDbError> {
                let collection = get_collection()?;
                let count = collection.count_documents(filter.into())
                    .await
                    .map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(e.to_string()))?;
                Ok(count)
            }

            async fn exists(
                filter: impl Into<::mongodb::bson::Document> + Send,
            ) -> Result<bool, ::monoxide_core::error::conn_error::MongoDbError> {
                Ok(Self::find_one(filter).await?.is_some())
            }
        }
    };

    expanded.into()
}
