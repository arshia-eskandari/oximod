use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, Attribute, DeriveInput, Lit, LitStr };

#[derive(Default, Debug)]
/// Arguments for creating an index on a field in a MongoDB collection.
///
/// This struct is populated from the `#[index(...)]` attribute
/// and specifies the behavior of the index.
///
/// # Fields
///
/// - `unique`: (Optional) Whether the index enforces a unique constraint.
///   - If `true`, MongoDB will reject documents that cause duplicate values for the indexed field.
///   - Default: `false`
///
/// - `sparse`: (Optional) Whether the index skips documents that are missing the field.
///   - If `true`, documents that do not have the indexed field will not be included in the index.
///   - Default: `false`
///
/// - `name`: (Optional) The custom name for the index.
///   - Useful for identifying indexes manually.
///   - If not provided, MongoDB will generate a default name.
///
/// - `background`: (Optional) Whether the index is built in the background.
///   - If `true`, index creation does not block database operations.
///   - Default: `false`
///
/// # Example
///
/// ```rust
/// #[index(unique = true, sparse = true, name = "email_idx", background = true, order = -1)]
/// email: String,
/// ```
///
/// # Notes
/// - These fields **can be combined freely** — for example, you can have an index that is both `unique` and `sparse`.
/// - MongoDB allows combining `unique`, `sparse`, and `background`.
/// - The `name` field is just metadata and does not conflict with others.
///
struct IndexArgs {
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub name: Option<String>,
    pub background: Option<bool>,
    pub order: Option<i32>,
}

#[derive(Debug)]
struct IndexDefinition {
    field_name: String,
    args: IndexArgs,
}

fn parse_index_args(attr: &Attribute, field_name: String) -> syn::Result<IndexDefinition> {
    let mut args = IndexArgs::default();

    if attr.path().is_ident("index") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("unique") {
                args.unique = Some(true);
            } else if meta.path.is_ident("sparse") {
                args.sparse = Some(true);
            } else if meta.path.is_ident("background") {
                args.background = Some(true);
            } else if meta.path.is_ident("name") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.name = Some(lit_str.value());
                }
            } else if meta.path.is_ident("order") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.order = Some(lit_int.base10_parse()?);
                }
            }
            Ok(())
        })?;
    }

    Ok(IndexDefinition { field_name, args })
}

#[proc_macro_derive(Model, attributes(db, collection, index))]
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
    let mut index_definitions = Vec::new();

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


    if let syn::Data::Struct(data_struct) = &input.data {
        for field in data_struct.fields.iter() {
            for attr in &field.attrs {
                if attr.path().is_ident("index") {
                    if let Some(ident) = &field.ident {
                        let field_name = ident.to_string();
                        let index_args = parse_index_args(attr, field_name)
                            .expect("could not parse index args");

                        index_definitions.push(index_args); // <-- COLLECT
                    }
                }
            }
        }
    }

    let index_models = index_definitions.iter().map(|index_def| {
        let field = &index_def.field_name;
        let order = index_def.args.order.unwrap_or(1);

        let unique = match index_def.args.unique {
            Some(val) => quote! { Some(#val) },
            None => quote! { None },
        };

        let sparse = match index_def.args.sparse {
            Some(val) => quote! { Some(#val) },
            None => quote! { None },
        };

        let background = match index_def.args.background {
            Some(val) => quote! { Some(#val) },
            None => quote! { None },
        };

        let name = match &index_def.args.name {
            Some(val) => quote! { Some(#val.to_string()) },
            None => quote! { None },
        };

        quote! {
            ::oximod::_mongodb::IndexModel::builder()
                .keys(::oximod::_mongodb::bson::doc! { #field: #order })
                .options(
                    ::oximod::_mongodb::options::IndexOptions::builder()
                        .unique(#unique)
                        .sparse(#sparse)
                        .background(#background)
                        .name(#name)
                        .build()
                )
                .build()
        }
    });


    let expanded =
        quote! {

        impl #name {
            fn _get_collection() -> Result<
                ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>, 
                ::oximod::_error::oximod_error::OximodError
            > {
                let client = ::oximod::_feature::conn::client::get_global_client()?;
                let db = client.database(#db);
                Ok(db.collection::<::oximod::_mongodb::bson::Document>(#collection))
            }
    
            async fn _create_indexes(
                collection: &::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>
            ) -> Result<(), ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;
    
                let indexes = vec![
                    #(#index_models),*
                ];
    
                if !indexes.is_empty() {
                    collection.create_indexes(indexes).await.map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to create indexes on the collection."
                        )
                    })?;
                }
    
                Ok(())
            }
        }

        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            async fn save(&self) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::_get_collection()?;
                Self::_create_indexes(&collection).await?; 
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
                let collection = Self::_get_collection()?;
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
