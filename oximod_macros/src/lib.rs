mod default;
mod helpers;
mod index;
#[macro_use]
mod parsers;
mod hooks_macro;
mod model_macro;
mod validate;

use helpers::{FieldTokenStreams, ModelAttrs, collect_model_attrs, generate_field_tokens};
use hooks_macro::generate_hooks_token;
use model_macro::generate_model_token;
use proc_macro::TokenStream;
use proc_macro2::Span;
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

    let hooks_token = generate_hooks_token(name);
    let model_token = generate_model_token(name, &db, &collection);

    let expanded = quote! {
        static #index_once_async_ident: ::oximod::_helpers::once_async::OnceAsync =
            ::oximod::_helpers::once_async::OnceAsync::new_with_options(
                Some(#index_max_retries),
                Some(::std::time::Duration::from_secs(#index_max_init_seconds as u64)),
            );

        impl #name {
            #[inline]
            fn validate(&self) -> Result<(), ::oximod::_error::oximod_error::OxiModError> {
                #(#validations)*
                Ok(())
            }

            #[doc(hidden)]
            #[cold]
            #[inline(never)]
            async fn _create_indexes(
                collection: &::oximod::_mongodb::Collection<Self>
            ) -> Result<(), ::oximod::_error::oximod_error::OxiModError> {

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
                                    ::oximod::_error::oximod_error::OxiModError::index("Failed to create indexes for collection", e)
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
                    ::oximod::_error::oximod_error::OxiModError,
            > {
                self.validate()?;
                let collection = Self::get_collection_from(client)?;
                Self::_create_indexes(&collection).await?;

                let result = collection.insert_one(self).await.map_err(|e|
                    ::oximod::_error::oximod_error::OxiModError::connection(
                        "Failed to insert document into MongoDB collection",
                        e,
                    )
                )?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err(
                        ::oximod::_error::oximod_error::OxiModError::validation(
                            "MongoDB returned a non-ObjectId inserted_id"
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

        #hooks_token
        #model_token

    };

    expanded.into()
}
