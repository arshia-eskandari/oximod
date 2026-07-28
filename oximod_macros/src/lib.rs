mod default;
mod helpers;
mod index;
#[macro_use]
mod parsers;
mod model_macro;
mod query;
mod validate;

use helpers::{FieldTokenStreams, ModelAttrs, collect_model_attrs, generate_field_tokens};
use model_macro::generate_model_token;
use proc_macro::TokenStream;
use proc_macro2::Span;
use query::generate_query_tokens;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

#[proc_macro_derive(
    Model,
    attributes(
        db,
        collection,
        index,
        validate,
        default,
        document_id_setter_ident,
        index_max_retries,
        index_max_init_seconds,
        hooks
    )
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
    let index_once_async_ident = Ident::new(&format!("_INDEX_INIT_{name}"), Span::call_site());

    let ModelAttrs {
        collection,
        db,
        index_max_retries,
        index_max_init_seconds,
        document_id_setter_ident,
        hooks,
    } = match collect_model_attrs(&input.attrs) {
        Ok(vals) => vals,
        Err(e) => return e.into(),
    };

    let FieldTokenStreams {
        indexes,
        validations,
        inits,
        setters,
    } = match generate_field_tokens(&input, &document_id_setter_ident) {
        Ok(token_streams) => token_streams,
        Err(e) => return e.into(),
    };

    let query_tokens = match generate_query_tokens(&input) {
        Ok(tokens) => tokens,
        Err(error) => return error.into(),
    };

    let model_token = generate_model_token(name, &db, &collection, hooks, validations);

    let expanded = quote! {
        static #index_once_async_ident: ::oximod::_helpers::once_async::OnceAsync =
            ::oximod::_helpers::once_async::OnceAsync::new_with_options(
                Some(#index_max_retries),
                Some(::std::time::Duration::from_secs(#index_max_init_seconds as u64)),
            );

        impl #name {
            #[doc(hidden)]
            #[cold]
            #[inline(never)]
            async fn _create_indexes(
                collection: &::oximod::_mongodb::Collection<Self>
            ) -> Result<(), ::oximod::OxiModError> {

                #index_once_async_ident
                    .run_once(|| async move {
                        let indexes = vec![
                            #(#indexes),*
                        ];

                        if !indexes.is_empty() {
                            collection
                                .create_indexes(indexes)
                                .await
                                .map_err(|e|
                                    ::oximod::OxiModError::index("Failed to create indexes for collection", e)
                                )?;
                        }

                        Ok(())
                    })
                    .await
            }

            async fn __oximod_insert_with_client(
                &self,
                client: &::oximod::_mongodb::Client,
            ) -> Result<
                    ::oximod::_mongodb::bson::oid::ObjectId,
                    ::oximod::OxiModError,
            > {
                self.validate()?;
                let collection = Self::get_collection_from(client)?;
                Self::_create_indexes(&collection).await?;

                let result = collection.insert_one(self).await.map_err(|e|
                    ::oximod::OxiModError::connection(
                        "Failed to insert document into MongoDB collection",
                        e,
                    )
                )?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err(
                        ::oximod::OxiModError::database(
                            "MongoDB returned a non-ObjectId inserted_id",
                            ::std::io::Error::other("inserted_id was not an ObjectId"),
                        )
                    )
                }
            }

            #[inline]
            pub fn new() -> Self {
                #name {
                    #(#inits),*
                }
            }

            #(
                #[inline]
                #setters
            )*
        }

        impl ::std::default::Default for #name {
            fn default() -> Self { Self::new() }
        }

        #model_token

        #query_tokens

    };

    expanded.into()
}

#[proc_macro_derive(EmbeddedDocument, attributes(serde))]
pub fn derive_embedded_document(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match query::generate_embedded_document_tokens(&input) {
        Ok(tokens) | Err(tokens) => tokens.into(),
    }
}
