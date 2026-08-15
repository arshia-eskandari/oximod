//! Procedural macros for OxiMod models.
//!
//! This crate provides the `Model` derive macro. Its expansion is assembled
//! from focused internal modules responsible for:
//!
//! - builder setters and defaults;
//! - model and field attribute parsing;
//! - validation;
//! - MongoDB indexes;
//! - collection persistence and lifecycle hooks;
//! - typed query and nested-field schemas.
//!
//! Collection-backed models receive the complete persistence and query API.
//! Models marked with `#[model(embedded)]` receive only behavior that is
//! meaningful without an independent MongoDB collection.

mod default;
mod helpers;
mod index;
mod model_macro;
mod parsers;
mod query;
mod validate;

use helpers::{
    CollectionAttrs, FieldTokenStreams, ModelAttrs, ModelKind, collect_model_attrs,
    generate_field_tokens,
};
use model_macro::{generate_collection_model_token, generate_model_token};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
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

    expand_model(&input).unwrap_or_else(|error| error).into()
}

/// Expands a parsed `Model` derive input.
///
/// Keeping expansion separate from the procedural-macro entry point allows the
/// complete orchestration layer to be tested using `proc_macro2::TokenStream`
/// without constructing compiler-owned `proc_macro::TokenStream` values.
///
/// # Errors
///
/// Returns compile-error tokens when model or field attributes are invalid,
/// the target declaration is unsupported, or an internally inconsistent model
/// configuration is encountered.
fn expand_model(input: &DeriveInput) -> Result<TokenStream2, TokenStream2> {
    let name = &input.ident;

    let ModelAttrs { kind, collection } = collect_model_attrs(&input.attrs)?;

    let document_id_setter_ident = collection
        .as_ref()
        .map(|attributes| attributes.document_id_setter_ident.as_str());

    let FieldTokenStreams {
        indexes,
        validations,
        inits,
        setters,
    } = generate_field_tokens(input, kind, document_id_setter_ident)?;

    let field_schema_tokens = generate_field_schema_tokens(input)?;

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

            let index_error_message =
                format!("Failed to create indexes for collection `{collection}`");

            let query_tokens = generate_query_tokens(input)?;

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
                    // The one generated source of truth for this model's
                    // declared `#[index(...)]` specifications: establishment
                    // and reconciliation must consume the same IndexModels.
                    #[doc(hidden)]
                    fn _declared_indexes()
                    -> Vec<::oximod::_mongodb::IndexModel> {
                        vec![
                            #(#indexes),*
                        ]
                    }

                    #[doc(hidden)]
                    #[cold]
                    #[inline(never)]
                    async fn _create_indexes(
                        collection:
                            &::oximod::_mongodb::Collection<Self>,
                    ) -> Result<(), ::oximod::OxiModError> {
                        #index_once_async_ident
                            .run_once(|| async move {
                                let indexes = Self::_declared_indexes();

                                if !indexes.is_empty() {
                                    collection
                                        .create_indexes(indexes)
                                        .await
                                        .map_err(|error| {
                                            ::oximod::_error::classify_driver_error(
                                                #index_error_message,
                                                ::oximod::_error::OperationDomain::Index,
                                                error,
                                            )
                                        })?;
                                }

                                Ok(())
                            })
                            .await
                    }

                    /// Establishes this model's declared `#[index(...)]`
                    /// specifications using the global client, without
                    /// saving any document.
                    ///
                    /// This reuses the same index-establishment machinery
                    /// and once-per-process state as save-triggered
                    /// initialization: a successful call is remembered for
                    /// this model type, repeated successful calls are
                    /// harmless no-ops, and a failed attempt may be retried
                    /// by a later call or save. This is establishment, not
                    /// drift synchronization: indexes dropped or changed
                    /// externally after a successful initialization are not
                    /// re-established during the same process.
                    ///
                    /// # Errors
                    ///
                    /// Returns an error when the global client has not been
                    /// initialized, an index error when MongoDB rejects
                    /// index creation, or a connection error when the
                    /// deployment cannot be reached.
                    pub async fn init_indexes() -> Result<(), ::oximod::OxiModError> {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection()?;

                        Self::_create_indexes(&collection).await
                    }

                    /// Establishes this model's declared `#[index(...)]`
                    /// specifications using the supplied client, without
                    /// saving any document.
                    ///
                    /// This is the explicit-client counterpart to
                    /// `init_indexes()` and does not require the global
                    /// client to be initialized. It shares the same
                    /// once-per-process establishment state and semantics.
                    ///
                    /// # Errors
                    ///
                    /// Returns an index error when MongoDB rejects index
                    /// creation, or a connection error when the deployment
                    /// cannot be reached.
                    pub async fn init_indexes_from(
                        client: &::oximod::_mongodb::Client,
                    ) -> Result<(), ::oximod::OxiModError> {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection_from(client)?;

                        Self::_create_indexes(&collection).await
                    }

                    /// Compares this model's declared `#[index(...)]`
                    /// specifications against the index metadata MongoDB
                    /// currently reports, using the global client.
                    ///
                    /// The inspection is read-only: it never creates the
                    /// collection or any index, and it is independent of the
                    /// once-per-process establishment state used by
                    /// `init_indexes()`. The result is a point-in-time
                    /// report: a concurrent process may change indexes at
                    /// any moment.
                    ///
                    /// Missing and mismatched declarations, together with
                    /// unmanaged indexes, are report data, not errors.
                    /// Requires the `listCollections` and `listIndexes`
                    /// privileges (MongoDB's built-in `read` role
                    /// suffices).
                    ///
                    /// # Errors
                    ///
                    /// Returns an error when the global client has not been
                    /// initialized, an index error when MongoDB rejects the
                    /// metadata inspection, or a connection error when the
                    /// deployment cannot be reached.
                    pub async fn check_indexes()
                    -> Result<::oximod::IndexDriftReport, ::oximod::OxiModError> {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection()?;

                        ::oximod::_index_reconciliation::check_indexes(
                            &collection,
                            Self::_declared_indexes(),
                        )
                        .await
                    }

                    /// Compares this model's declared `#[index(...)]`
                    /// specifications against the index metadata MongoDB
                    /// currently reports, using the supplied client.
                    ///
                    /// This is the explicit-client counterpart to
                    /// `check_indexes()` and does not require the global
                    /// client to be initialized. See `check_indexes()` for
                    /// the read-only, point-in-time semantics.
                    ///
                    /// # Errors
                    ///
                    /// Returns an index error when MongoDB rejects the
                    /// metadata inspection, or a connection error when the
                    /// deployment cannot be reached.
                    pub async fn check_indexes_from(
                        client: &::oximod::_mongodb::Client,
                    ) -> Result<::oximod::IndexDriftReport, ::oximod::OxiModError> {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection_from(client)?;

                        ::oximod::_index_reconciliation::check_indexes(
                            &collection,
                            Self::_declared_indexes(),
                        )
                        .await
                    }

                    /// Creates only the declared `#[index(...)]`
                    /// specifications that are currently missing from the
                    /// server, using the global client.
                    ///
                    /// Declarations reported mismatched and unmanaged
                    /// raw-driver indexes are never modified: this method
                    /// sends at most one `createIndexes` command for the
                    /// missing declarations and never drops, hides,
                    /// unhides, or otherwise alters an existing index. When
                    /// nothing is missing, no command is sent at all, so a
                    /// model with zero declared indexes never creates its
                    /// collection. The once-per-process establishment state
                    /// used by `init_indexes()` is neither consulted nor
                    /// modified.
                    ///
                    /// Creating an index is still operationally
                    /// consequential: index builds consume resources, a
                    /// unique index build fails when existing data violates
                    /// uniqueness, and a new TTL index can make already
                    /// expired documents immediately eligible for deletion.
                    /// Use this during controlled startup, deployment, or
                    /// maintenance workflows. Requires the `createIndex`
                    /// privilege in addition to the inspection privileges
                    /// (MongoDB's built-in `readWrite` role suffices).
                    ///
                    /// # Errors
                    ///
                    /// Returns an error when the global client has not been
                    /// initialized, an index error when MongoDB rejects the
                    /// inspection or creation, or a connection error when
                    /// the deployment cannot be reached. A failed creation
                    /// does not imply the server is unchanged; call
                    /// `check_indexes()` again for current state.
                    pub async fn create_missing_indexes()
                    -> Result<::oximod::IndexReconciliationReport, ::oximod::OxiModError>
                    {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection()?;

                        ::oximod::_index_reconciliation::create_missing_indexes(
                            &collection,
                            Self::_declared_indexes(),
                        )
                        .await
                    }

                    /// Creates only the declared `#[index(...)]`
                    /// specifications that are currently missing from the
                    /// server, using the supplied client.
                    ///
                    /// This is the explicit-client counterpart to
                    /// `create_missing_indexes()` and does not require the
                    /// global client to be initialized. See
                    /// `create_missing_indexes()` for the conservative
                    /// create-only semantics and operational caveats.
                    ///
                    /// # Errors
                    ///
                    /// Returns an index error when MongoDB rejects the
                    /// inspection or creation, or a connection error when
                    /// the deployment cannot be reached. A failed creation
                    /// does not imply the server is unchanged; call
                    /// `check_indexes_from()` again for current state.
                    pub async fn create_missing_indexes_from(
                        client: &::oximod::_mongodb::Client,
                    ) -> Result<::oximod::IndexReconciliationReport, ::oximod::OxiModError>
                    {
                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection_from(client)?;

                        ::oximod::_index_reconciliation::create_missing_indexes(
                            &collection,
                            Self::_declared_indexes(),
                        )
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
                                ::oximod::_error::classify_driver_error(
                                    "Failed to insert document into MongoDB collection",
                                    ::oximod::_error::OperationDomain::General,
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

                    // Unlike the client-based insert helper, this does not
                    // establish declared indexes: index creation is not part
                    // of the caller's transaction and must happen before
                    // transactional work begins.
                    #[doc(hidden)]
                    async fn __oximod_insert_with_session(
                        &self,
                        session: &mut ::oximod::_mongodb::ClientSession,
                    ) -> Result<
                        ::oximod::_mongodb::bson::oid::ObjectId,
                        ::oximod::OxiModError,
                    > {
                        <
                            Self as ::oximod::_feature::model::ModelCore<
                                ::oximod::_feature::model::Collection
                            >
                        >::validate(self)?;

                        let client = session.client();

                        let collection = <
                            Self as ::oximod::_feature::model::Model
                        >::get_collection_from(&client)?;

                        let result = collection
                            .insert_one(self)
                            .session(&mut *session)
                            .await
                            .map_err(|error| {
                                ::oximod::_error::classify_driver_error(
                                    "Failed to insert document into MongoDB collection",
                                    ::oximod::_error::OperationDomain::General,
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
            return Err(syn::Error::new_spanned(
                &input.ident,
                "internal OxiMod macro error: inconsistent model configuration",
            )
            .to_compile_error());
        }
    };

    Ok(quote! {
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
    })
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    use super::expand_model;

    #[test]
    fn derive_expansion_assembles_collection_only_features() {
        let input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,
                name: String,
            }
        };

        let generated = compact(expand_model(&input).expect("collection model should expand"));

        for expected in [
            "_INDEX_INIT_User",
            "ModelCore<::oximod::_feature::model::Collection>forUser",
            "FieldSchemaforUser",
            "QueryableforUser",
            "__oximod_insert_with_client",
            "_create_indexes",
        ] {
            assert!(
                generated.contains(expected),
                "expected `{expected}` in generated collection model: \
                 {generated}"
            );
        }
    }

    #[test]
    fn init_indexes_methods_are_generated_for_collection_models_only() {
        let collection_input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,

                #[index(unique)]
                name: String,
            }
        };

        let generated =
            compact(expand_model(&collection_input).expect("collection model should expand"));

        for expected in [
            "pubasyncfninit_indexes()",
            "pubasyncfninit_indexes_from(client:&::oximod::_mongodb::Client,)",
        ] {
            assert!(
                generated.contains(expected),
                "expected `{expected}` in generated collection model: \
                 {generated}"
            );
        }

        let embedded_input: DeriveInput = parse_quote! {
            #[model(embedded)]
            struct Address {
                city: String,
            }
        };

        let generated =
            compact(expand_model(&embedded_input).expect("embedded model should expand"));

        assert!(
            !generated.contains("init_indexes"),
            "embedded model unexpectedly received index initialization \
             methods: {generated}"
        );
    }

    #[test]
    fn reconciliation_methods_are_generated_for_collection_models_only() {
        let collection_input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,

                #[index(unique)]
                name: String,
            }
        };

        let generated =
            compact(expand_model(&collection_input).expect("collection model should expand"));

        for expected in [
            "pubasyncfncheck_indexes()->Result<::oximod::IndexDriftReport,::oximod::OxiModError>",
            "pubasyncfncheck_indexes_from(client:&::oximod::_mongodb::Client,)",
            "pubasyncfncreate_missing_indexes()->Result<::oximod::IndexReconciliationReport,\
             ::oximod::OxiModError>",
            "pubasyncfncreate_missing_indexes_from(client:&::oximod::_mongodb::Client,)",
            "::oximod::_index_reconciliation::check_indexes(&collection,Self::_declared_indexes(),)",
            "::oximod::_index_reconciliation::create_missing_indexes(&collection,\
             Self::_declared_indexes(),)",
        ] {
            assert!(
                generated.contains(expected),
                "expected `{expected}` in generated collection model: \
                 {generated}"
            );
        }

        let embedded_input: DeriveInput = parse_quote! {
            #[model(embedded)]
            struct Address {
                city: String,
            }
        };

        let generated =
            compact(expand_model(&embedded_input).expect("embedded model should expand"));

        for unexpected in ["check_indexes", "create_missing_indexes"] {
            assert!(
                !generated.contains(unexpected),
                "embedded model unexpectedly received `{unexpected}`: \
                 {generated}"
            );
        }
    }

    #[test]
    fn reconciliation_methods_never_touch_the_once_state() {
        let input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,

                #[index(unique)]
                name: String,
            }
        };

        let generated = compact(expand_model(&input).expect("collection model should expand"));

        // The once-per-process establishment static appears exactly twice:
        // its declaration and its single use inside `_create_indexes`. The
        // reconciliation methods are independent of it by construction.
        assert_eq!(
            generated.matches("_INDEX_INIT_User").count(),
            2,
            "the OnceAsync state should only be declared and used by \
             `_create_indexes`: {generated}"
        );
    }

    #[test]
    fn declared_indexes_helper_is_the_single_generated_index_source() {
        let input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,

                #[index(unique, name = "name_idx")]
                name: String,

                #[index(order = "-1")]
                age: i32,
            }
        };

        let generated = compact(expand_model(&input).expect("collection model should expand"));

        // The helper holds the generated IndexModel expressions...
        assert!(
            generated.contains("fn_declared_indexes()->Vec<::oximod::_mongodb::IndexModel>{vec![",),
            "expected the `_declared_indexes` helper in: {generated}"
        );
        for declaration in [
            "doc!{stringify!(name):1",
            ".unique(Some(true))",
            ".name(Some(\"name_idx\".to_string()))",
            "doc!{stringify!(age):-1",
        ] {
            assert!(
                generated.contains(declaration),
                "expected declaration tokens `{declaration}` in: {generated}"
            );
        }

        // ...establishment consumes it instead of rebuilding declarations...
        assert!(
            generated.contains("letindexes=Self::_declared_indexes();"),
            "expected `_create_indexes` to consume `_declared_indexes`: \
             {generated}"
        );

        // ...and the IndexModel constructor appears exactly once per declared
        // index, so no second generated declaration list can drift.
        assert_eq!(
            generated
                .matches("::oximod::_mongodb::IndexModel::builder()")
                .count(),
            2,
            "each declared index should be generated exactly once: {generated}"
        );
    }

    #[test]
    fn embedded_models_receive_no_declared_indexes_helper() {
        let input: DeriveInput = parse_quote! {
            #[model(embedded)]
            struct Address {
                city: String,
            }
        };

        let generated = compact(expand_model(&input).expect("embedded model should expand"));

        assert!(
            !generated.contains("_declared_indexes"),
            "embedded model unexpectedly received `_declared_indexes`: \
             {generated}"
        );
    }

    #[test]
    fn index_creation_errors_name_the_collection() {
        let input: DeriveInput = parse_quote! {
            #[db("app")]
            #[collection("users")]
            struct User {
                _id: Option<
                    ::oximod::_mongodb::bson::oid::ObjectId
                >,

                #[index(unique)]
                name: String,
            }
        };

        let generated = compact(expand_model(&input).expect("collection model should expand"));

        assert!(
            generated.contains("\"Failedtocreateindexesforcollection`users`\""),
            "index error message should include the collection name: \
             {generated}"
        );
    }

    #[test]
    fn derive_expansion_omits_collection_features_for_embedded_models() {
        let input: DeriveInput = parse_quote! {
            #[model(embedded)]
            struct Address {
                city: String,
            }
        };

        let generated = compact(expand_model(&input).expect("embedded model should expand"));

        for expected in [
            "ModelCore<::oximod::_feature::model::Embedded>forAddress",
            "FieldSchemaforAddress",
            "pubfnnew()->Self",
        ] {
            assert!(
                generated.contains(expected),
                "expected `{expected}` in generated embedded model: \
                 {generated}"
            );
        }

        for unexpected in [
            "_INDEX_INIT_Address",
            "QueryableforAddress",
            "__oximod_insert_with_client",
            "_create_indexes",
        ] {
            assert!(
                !generated.contains(unexpected),
                "embedded model unexpectedly contained `{unexpected}`: \
                 {generated}"
            );
        }
    }

    #[test]
    fn derive_expansion_preserves_compile_error_diagnostics() {
        let input: DeriveInput = parse_quote! {
            #[db("app")]
            struct User {
                name: String,
            }
        };

        let error = expand_model(&input)
            .expect_err("missing collection should fail")
            .to_string();

        assert!(
            error.contains("compile_error"),
            "error should remain compile-error tokens: {error}"
        );
        assert!(
            error.contains("Missing") && error.contains("collection_name"),
            "missing-collection diagnostic should be retained: {error}"
        );
    }

    fn compact(tokens: proc_macro2::TokenStream) -> String {
        tokens.to_string().replace(' ', "")
    }
}
