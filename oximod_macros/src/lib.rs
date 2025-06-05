mod index;
mod validate;
mod default;
use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, DeriveInput, LitStr };
use index::{ parse_index_args, generate_index_model_tokens };
use validate::{ parse_validate_args, generate_validate_model_tokens };
use default::{ parse_default_args, maybe_push_id_setter, push_field_setters };

#[proc_macro_derive(
    Model,
    attributes(db, collection, index, validate, default, document_id_setter_ident)
)]
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
    let mut validate_definitions = Vec::new();
    let mut default_definitions = Vec::new();
    let mut all_fields: Vec<(syn::Ident, syn::Type)> = Vec::new();
    let mut has_id_attr = false;
    let mut setters = Vec::new();

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
            if let Some(ident) = &field.ident {
                all_fields.push((ident.clone(), field.ty.clone()));
                for attr in &field.attrs {
                    let field_name = ident.to_string();
                    if field_name == "_id".to_string() {
                        has_id_attr = true;
                    }
                    if attr.path().is_ident("index") {
                        let index_args = parse_index_args(attr, field_name.clone()).expect(
                            "could not parse index args"
                        );
                        index_definitions.push(index_args); // <-- COLLECT
                    } else if attr.path().is_ident("validate") {
                        let validate_definition = parse_validate_args(
                            attr,
                            field_name.clone()
                        ).expect("could not parse validate args");
                        validate_definitions.push(validate_definition);
                    } else if attr.path().is_ident("default") {
                        let def = parse_default_args(attr, ident).expect(
                            "could not parse default args"
                        );
                        default_definitions.push(def);
                    }
                }
            }
        }
    }

    let index_models = index_definitions
        .iter()
        .map(|index_def| generate_index_model_tokens(index_def));

    let validations = validate_definitions
        .iter()
        .flat_map(|validate_def| generate_validate_model_tokens(validate_def));

    let default_inits = default_definitions.iter().map(|def| {
        let ident = &def.field_ident;
        let expr = &def.default_expr;
        quote! { #ident: #expr, }
    });

    let default_idents: HashSet<String> = default_definitions
        .iter()
        .map(|d| d.field_ident.to_string())
        .collect();

    let other_inits = all_fields
        .iter()
        .filter(|(ident, _ty)| !default_idents.contains(&ident.to_string()))
        .map(|(ident, _ty)| {
            quote! { #ident: Default::default(), }
        });

    maybe_push_id_setter(has_id_attr, &input.attrs, &mut setters);
    push_field_setters(&all_fields, &mut setters);

    let expanded =
        quote! {

        impl #name {
            fn validate(&self) -> Result<(), ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;
                #(#validations)*
                Ok(())
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
                            ::oximod::_error::oximod_error::OximodError::IndexError(e.to_string()),
                            "Failed to create indexes on the collection."
                        )
                    })?;
                }
    
                Ok(())
            }

            pub fn new() -> Self {
                #name {
                    #(#default_inits)*
                    #(#other_inits)*
                }
            }
        
            #(#setters)*
        }

        impl ::std::default::Default for #name {
            fn default() -> Self { Self::new() }
        }

        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            fn get_collection() -> Result<
                ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>, 
                ::oximod::_error::oximod_error::OximodError
            > {
                let client = ::oximod::_feature::conn::client::get_global_client()?;
                let db = client.database(#db);
                Ok(db.collection::<::oximod::_mongodb::bson::Document>(#collection))
            }
            
            async fn save(&self) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OximodError> {
                self.validate()?; 
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
                let collection = Self::get_collection()?;
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
