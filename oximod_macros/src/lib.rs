use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, DeriveInput, LitStr };

#[proc_macro_derive(Model, attributes(db, collection))]
/// Procedural macro to derive the `Model` trait for mongodb schema support.
///
/// This macro enables automatic implementation of the `Model` trait, allowing
/// CRUD operations and schema-based mongodb interaction.
///
/// # Required Attributes
///
/// - `#[db("your_database_name")]`: Specifies the database name.
/// - `#[collection("your_collection_name")]`: Specifies the collection name.
///
/// # Example
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize, Debug)]
/// #[db("test")]
/// #[collection("users")]
/// pub struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     name: String,
///     age: i32,
///     active: bool,
/// }
/// ```
///
/// Once derived, you can use methods like `.save()`, `.find()`, `.update_one()`, `.delete()`, etc.,
/// provided by the `Model` trait.
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
            ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>, 
            ::oximod::_error::oximod_error::OximodError
        > {
            let client = ::oximod::_feature::conn::client::get_global_client()?;
            let db = client.database(#db);
            Ok(db.collection::<::oximod::_mongodb::bson::Document>(#collection))
        }

        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            async fn save(&self) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;

                let document = ::oximod::_mongodb::bson::to_document(&self).map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                        "Failed to serialize model. Are all field types supported by bson::to_document()?"
                    )
                })?;

                let result = collection.insert_one(document).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                        "Failed to insert document. Check if the mongodb server is reachable and the collection exists."
                    )
                })?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err( ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::SerializationError("inserted_id is not an ObjectId".to_string()),
                        "Expected inserted_id to be an ObjectId but received something else. This may happen if you're using a custom _id."
                    ))
                }
            }

            async fn update(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .update_many(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to update documents. Check your update operators and filter structure."
                        )
                    })?;

                Ok(result)
            }

            async fn update_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .update_one(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to update a document. Make sure your update syntax is valid and the filter matches at least one document."
                        )
                    })?;

                Ok(result)
            }

            async fn delete(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .delete_many(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to delete documents. Ensure your filter is valid and matches the correct documents."
                        )
                    })?;

                Ok(result)
            }

            async fn delete_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .delete_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to delete a single document. Ensure your filter is valid and matches the correct document."
                        )
                    })?;

                Ok(result)
            }

            async fn find(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<Vec<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let mut cursor = collection
                    .find(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to execute find query. Double-check your filter syntax or collection state."
                        )
                    })?;

                let mut results = vec![];

                while let Some(doc) = ::oximod::_futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc.map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Cursor failed to retrieve a document. This may indicate a deserialization or network error mid-stream."
                        )
                    })?;

                    let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                            "Failed to deserialize document into model. Check field types and optionality."
                        )
                    })?;

                    results.push(parsed);
                }

                Ok(results)
            }

            async fn find_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .find_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to run find_one query. Ensure your filter is structured properly and the collection is accessible."
                        )
                    })?;

                match result {
                    Some(doc) => {
                        let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                            ::oximod::_attach_printables!(
                                ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                                "Could not deserialize document into model. Check for type mismatches or missing #[serde] attributes."
                            )
                        })?;
                        Ok(Some(parsed))
                    }
                    None => Ok(None),
                }
            }

            async fn find_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                use ::oximod::_error::printable::Printable;

                Self::find_one(::oximod::_mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to find document by _id. Confirm the ID is valid and the document exists."
                    )
                })
            }

            async fn update_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::update_one(::oximod::_mongodb::bson::doc! { "_id": id }, update).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to update document by _id. Check if the document exists and if your update operators are valid."
                    )
                })
            }

            async fn delete_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::delete_one(::oximod::_mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to delete document by _id. Ensure the ID is correct and that the document exists."
                    )
                })
            }

            async fn count(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<u64, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;

                let count = collection
                    .count_documents(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to count documents. Make sure the filter is well-formed and the collection is accessible."
                        )
                    })?;

                Ok(count)
            }

            async fn exists(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<bool, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::find_one(filter).await
                    .map(|opt| opt.is_some())
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            e,
                            "Failed to check document existence. Make sure your filter is valid and your connection is healthy."
                        )
                    })
            }

            async fn clear() -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;

                let result = collection
                    .delete_many(::oximod::_mongodb::bson::doc! {})
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to clear the collection. Ensure the mongodb connection is valid and the collection is writable."
                        )
                    })?;

                Ok(result)
            }

            async fn aggregate(
                pipeline: impl Into<Vec<::oximod::_mongodb::bson::Document>> + Send
            ) -> Result<::oximod::_mongodb::Cursor<oximod::_mongodb::bson::Document>, ::oximod::_error::oximod_error::OximodError> {
                let collection = get_collection()?;
                use ::oximod::_error::printable::Printable;

                let result = collection.aggregate(pipeline.into()).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::AggregationError(e.to_string()),
                            "Failed to aggregate. Ensure your pipeline is valid and the collection is readable."
                    )
                })?;

                Ok(result)
            }
        }
    };

    expanded.into()
}
