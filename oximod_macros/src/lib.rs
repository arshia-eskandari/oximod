mod default;
mod helpers;
mod index;
#[macro_use]
mod parsers;
mod model_macro;
mod validate;

use helpers::{collect_model_attrs, generate_field_tokens};
use model_macro::generate_model_token;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

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

    let mut setters = Vec::new();
    let mut validations = Vec::new();
    let mut indexes = Vec::new();
    let mut inits = Vec::new();

    let (collection, db, index_max_retries, index_max_init_seconds, document_id_setter_ident) =
        match collect_model_attrs(&input.attrs) {
            Ok(vals) => vals,
            Err(err_tokens) => return err_tokens.into(),
        };

    if let Err(e) = generate_field_tokens(
        &input,
        &mut indexes,
        &mut validations,
        &mut inits,
        &mut setters,
        &document_id_setter_ident,
    ) {
        return e.into();
    }

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
                collection: &::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>
            ) -> Result<(), ::oximod::_error::oximod_error::OxiModError> {

                #index_once_async_ident
                    .run_once(|| async move {
                        let indexes = vec![
                            #(#indexes),*
                        ];

                        if !indexes.is_empty() {
                            collection.create_indexes(indexes).await.map_err(|e| {
                                ::oximod::_attach_printables!(
                                    ::oximod::_error::oximod_error::OxiModError::IndexError(::std::format!("{e}")),
                                    @static "Failed to create indexes on the collection."
                                )
                            })?;
                        }

                        Ok(())
                    })
                    .await
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

    };

    expanded.into()
}
