mod default;
mod helpers;
mod index;
#[macro_use]
mod parsers;
mod model_macro;
mod query;
mod validate;

use helpers::{
    CollectionAttrs, FieldTokenStreams, ModelAttrs, ModelKind, collect_model_attrs,
    generate_field_tokens,
};
use model_macro::{generate_collection_model_token, generate_model_token};
use proc_macro::TokenStream;
use proc_macro2::Span;
use query::{generate_field_schema_tokens, generate_query_tokens};
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

#[proc_macro_derive(
    Model,
    attributes(
        model,
        db,
        collection,
        index,
        validate,
        default,
        document_id_setter_ident,
        index_max_retries,
        index_max_init_seconds,
        hooks,
        serde
    )
)]
/// Procedural macro for generating OxiMod model functionality.
///
/// By default, the derived type is a collection-backed model. Collection-backed
/// models support builders, defaults, validation, typed queries, indexes,
/// lifecycle hooks, and MongoDB persistence.
///
/// Use `#[model(embedded)]` to generate an embedded model. Embedded models
/// support builders, defaults, validation, and typed nested-field access, but
/// do not receive collection access, indexes, hooks, queries, or persistence
/// methods.
///
/// # Collection-backed models
///
/// Collection-backed models require:
///
/// - `#[db("your_database_name")]`
/// - `#[collection("your_collection_name")]`
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize, Debug)]
/// #[db("app")]
/// #[collection("users")]
/// pub struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     name: String,
///     address: Address,
/// }
/// ```
///
/// # Embedded models
///
/// Embedded models are declared with `#[model(embedded)]`:
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize, Debug)]
/// #[model(embedded)]
/// pub struct Address {
///     street: String,
///     city: String,
/// }
/// ```
///
/// The following attributes are not supported for embedded models:
///
/// - `#[db(...)]`
/// - `#[collection(...)]`
/// - `#[hooks]`
/// - `#[index(...)]`
/// - `#[document_id_setter_ident(...)]`
/// - `#[index_max_retries(...)]`
/// - `#[index_max_init_seconds(...)]`
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let ModelAttrs { kind, collection } = match collect_model_attrs(&input.attrs) {
        Ok(attributes) => attributes,
        Err(error) => return error.into(),
    };

    let document_id_setter_ident = collection
        .as_ref()
        .map(|attributes| attributes.document_id_setter_ident.as_str());

    let FieldTokenStreams {
        indexes,
        validations,
        inits,
        setters,
    } = match generate_field_tokens(&input, kind, document_id_setter_ident) {
        Ok(token_streams) => token_streams,
        Err(error) => return error.into(),
    };

    let field_schema_tokens = match generate_field_schema_tokens(&input) {
        Ok(tokens) => tokens,
        Err(error) => return error.into(),
    };

    let model_token = generate_model_token(name, kind, validations);

    let collection_tokens = match (kind, collection) {
        (
            ModelKind::Collection,
            Some(CollectionAttrs {
                collection,
                db,
                index_max_retries,
                index_max_init_seconds,
                document_id_setter_ident: _,
                hooks,
            }),
        ) => {
            let index_once_async_ident =
                Ident::new(&format!("_INDEX_INIT_{name}"), Span::call_site());

            let query_tokens = match generate_query_tokens(&input) {
                Ok(tokens) => tokens,
                Err(error) => return error.into(),
            };

            let collection_model_token =
                generate_collection_model_token(name, &db, &collection, hooks);

            quote! {
                static #index_once_async_ident:
                    ::oximod::_helpers::once_async::OnceAsync =
                    ::oximod::_helpers::once_async::OnceAsync::new_with_options(
                        Some(#index_max_retries),
                        Some(
                            ::std::time::Duration::from_secs(
                                #index_max_init_seconds as u64,
                            ),
                        ),
                    );

                impl #name {
                    #[doc(hidden)]
                    #[cold]
                    #[inline(never)]
                    async fn _create_indexes(
                        collection:
                            &::oximod::_mongodb::Collection<Self>,
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
                                        .map_err(|error| {
                                            ::oximod::OxiModError::index(
                                                "Failed to create indexes for collection",
                                                error,
                                            )
                                        })?;
                                }

                                Ok(())
                            })
                            .await
                    }

                    #[doc(hidden)]
                    async fn __oximod_insert_with_client(
                        &self,
                        client: &::oximod::_mongodb::Client,
                    ) -> Result<
                        ::oximod::_mongodb::bson::oid::ObjectId,
                        ::oximod::OxiModError,
                    > {
                        <
                            Self as ::oximod::_feature::model::ModelCore<
                                ::oximod::_feature::model::Collection
                            >
                        >::validate(self)?;

                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection_from(client)?;

                        Self::_create_indexes(&collection).await?;

                        let result = collection
                            .insert_one(self)
                            .await
                            .map_err(|error| {
                                ::oximod::OxiModError::connection(
                                    "Failed to insert document into MongoDB collection",
                                    error,
                                )
                            })?;

                        match result.inserted_id.as_object_id() {
                            Some(id) => Ok(id),

                            None => Err(
                                ::oximod::OxiModError::database(
                                    "MongoDB returned a non-ObjectId inserted_id",
                                    ::std::io::Error::other(
                                        "inserted_id was not an ObjectId",
                                    ),
                                ),
                            ),
                        }
                    }
                }

                #collection_model_token

                #query_tokens
            }
        }

        (ModelKind::Embedded, None) => {
            quote! {}
        }

        _ => {
            return syn::Error::new_spanned(
                &input.ident,
                "internal OxiMod macro error: inconsistent model configuration",
            )
            .to_compile_error()
            .into();
        }
    };

    let expanded = quote! {
        impl #name {
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
            fn default() -> Self {
                Self::new()
            }
        }

        #model_token

        #field_schema_tokens

        #collection_tokens
    };

    expanded.into()
}
