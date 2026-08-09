//! Field-level token generation for derived models.
//!
//! This module processes every named model field and collects the generated
//! token streams for:
//!
//! - fluent builder setters;
//! - default-value initialization;
//! - field validation;
//! - collection index definitions.
//!
//! Collection models receive special `_id` handling and may define indexes.
//! Embedded models treat `_id` as an ordinary field and reject indexes.

use crate::{
    default::{push_field_setter, push_id_setter},
    helpers::ModelKind,
    index::generate_index_model_tokens,
    parsers::{parse_default_expr, parse_index_args, parse_validate_args},
    validate::generate_validate_model_tokens,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub struct FieldTokenStreams {
    pub indexes: Vec<TokenStream>,
    pub validations: Vec<TokenStream>,
    pub inits: Vec<TokenStream>,
    pub setters: Vec<TokenStream>,
}

/// Processes all fields of a struct annotated with `#[derive(Model)]`,
/// expanding supported field-level attributes into token streams used
/// for the generated implementation.
///
/// The generated field behavior depends on the model's [`ModelKind`].
/// Collection-backed and embedded models both receive fluent setters,
/// validation, and default-value initialization. Indexes and the special
/// document ID setter are available only to collection-backed models.
///
/// Specifically:
/// - Inserts setter functions for each field.
/// - Special-cases `_id` for collection-backed models by generating the
///   configured document ID setter.
/// - Treats `_id` as an ordinary field for embedded models.
/// - Expands `#[index(...)]` attributes into index model tokens for
///   collection-backed models.
/// - Rejects `#[index(...)]` attributes on embedded models because embedded
///   documents do not own an independent MongoDB collection.
/// - Expands `#[validate(...)]` attributes into validation tokens for both
///   collection-backed and embedded models.
/// - Expands `#[default = "..."]` attributes into custom initialization
///   expressions for both model kinds, falling back to `Default::default()`
///   otherwise.
/// - Collects all generated tokens into the provided vectors for setters,
///   indexes, validations, and initializations.
///
/// # Parameters
///
/// - `input`: The parsed input for the type deriving `Model`.
/// - `kind`: Whether the model is collection-backed or embedded.
/// - `document_id_setter_ident`: The configured document ID setter name for a
///   collection-backed model. This is `None` for an embedded model.
///
/// # Errors
///
/// Returns a `TokenStream` error if:
///
/// - the macro target is not a struct,
/// - a field attribute fails to parse,
/// - an embedded model declares an `#[index(...)]` attribute,
/// - a model declares more than one text or text-implying index,
/// - a model declares the same literal `#[index(name = "...")]` twice,
/// - or a collection-backed model is missing its document ID setter
///   configuration.
pub fn generate_field_tokens(
    input: &DeriveInput,
    kind: ModelKind,
    document_id_setter_ident: Option<&str>,
) -> Result<FieldTokenStreams, TokenStream> {
    let mut indexes = Vec::new();
    let mut validations = Vec::new();
    let mut inits = Vec::new();
    let mut setters = Vec::new();

    // Cross-field `#[index]` declaration state. MongoDB permits at most one
    // text index per collection and rejects duplicate index names at creation
    // time, so both conflicts are reported at compile time instead.
    let mut text_index_field: Option<String> = None;
    let mut declared_index_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let data_struct = match &input.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Model can only be derived for structs.",
            )
            .to_compile_error());
        }
    };

    for field in data_struct.fields.iter() {
        if let Some(ident) = &field.ident {
            if ident == "_id" && kind == ModelKind::Collection {
                let document_id_setter_ident =
                    document_id_setter_ident.ok_or_else(|| {
                        syn::Error::new_spanned(
                            ident,
                            "internal OxiMod macro error: collection model is missing its document ID setter name",
                        )
                        .to_compile_error()
                    })?;

                push_id_setter(&mut setters, document_id_setter_ident)?;
            } else {
                push_field_setter(ident, &field.ty, &mut setters);
            }

            let mut init_expr: TokenStream = quote! { Default::default() };

            for attr in &field.attrs {
                if attr.path().is_ident("index") {
                    if kind == ModelKind::Embedded {
                        return Err(syn::Error::new_spanned(
                            attr,
                            "`index` is not supported on embedded models",
                        )
                        .to_compile_error());
                    }

                    let index_args = parse_index_args(attr).map_err(|error| {
                        syn::Error::new_spanned(attr, format!("Invalid #[index]: {error}"))
                            .to_compile_error()
                    })?;

                    if index_args.is_text_implying() {
                        if let Some(first_field) = &text_index_field {
                            return Err(syn::Error::new_spanned(
                                attr,
                                format!(
                                    "conflicting #[index] declarations: field `{ident}` declares \
                                     a text or text-implying index, but field `{first_field}` \
                                     already declares one; MongoDB allows at most one text index \
                                     per collection"
                                ),
                            )
                            .to_compile_error());
                        }

                        text_index_field = Some(ident.to_string());
                    }

                    if let Some(name) = &index_args.name
                        && let Some(first_field) =
                            declared_index_names.insert(name.clone(), ident.to_string())
                    {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!(
                                "duplicate #[index] name `{name}`: an index with this name \
                                 is already declared on field `{first_field}`"
                            ),
                        )
                        .to_compile_error());
                    }

                    let index_token = generate_index_model_tokens(ident, index_args);

                    indexes.push(index_token);
                } else if attr.path().is_ident("validate") {
                    let validate_args = parse_validate_args(attr).map_err(|error| {
                        syn::Error::new_spanned(attr, format!("Invalid #[validate]: {error}"))
                            .to_compile_error()
                    })?;

                    let validation_tokens = generate_validate_model_tokens(
                        &input.ident,
                        ident,
                        &field.ty,
                        validate_args,
                    );

                    validations.extend(validation_tokens);
                } else if attr.path().is_ident("default") {
                    let default_expr = parse_default_expr(attr).map_err(|error| {
                        syn::Error::new_spanned(attr, format!("Invalid #[default]: {error}"))
                            .to_compile_error()
                    })?;

                    init_expr = quote! { (#default_expr).into() };
                }
            }

            inits.push(quote! {
                #ident: #init_expr
            });
        }
    }

    Ok(FieldTokenStreams {
        indexes,
        validations,
        inits,
        setters,
    })
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{DeriveInput, parse_quote};

    use crate::helpers::ModelKind;

    use super::generate_field_tokens;

    #[test]
    fn collection_models_generate_special_id_and_regular_setters() {
        let input: DeriveInput = parse_quote! {
            struct User {
                _id: Option<::oximod::_mongodb::bson::oid::ObjectId>,
                name: String,
                nickname: Option<String>,
            }
        };

        let fields = generate_field_tokens(&input, ModelKind::Collection, Some("with_id"))
            .expect("collection field tokens should generate");

        assert_eq!(fields.setters.len(), 3);
        assert_eq!(fields.inits.len(), 3);
        assert!(fields.indexes.is_empty());
        assert!(fields.validations.is_empty());

        let id_setter = compact(&fields.setters[0]);

        assert!(
            id_setter.contains("pubfnwith_id("),
            "configured ID setter should be generated: {id_setter}"
        );
        assert!(
            id_setter.contains("self._id=Some(id);"),
            "ID setter should assign the ObjectId: {id_setter}"
        );

        let name_setter = compact(&fields.setters[1]);

        assert!(
            name_setter.contains("pubfnname<T>"),
            "regular field setter should be generated: {name_setter}"
        );
        assert!(
            name_setter.contains("self.name=val.into();"),
            "regular setter should assign the converted value: \
             {name_setter}"
        );

        let nickname_setter = compact(&fields.setters[2]);

        assert!(
            nickname_setter.contains("pubfnnickname<T>"),
            "optional field setter should be generated: \
             {nickname_setter}"
        );
        assert!(
            nickname_setter.contains("self.nickname=Some(val.into());"),
            "optional setter should wrap its value in Some: \
             {nickname_setter}"
        );
    }

    #[test]
    fn embedded_models_treat_id_as_an_ordinary_field() {
        let input: DeriveInput = parse_quote! {
            struct Address {
                _id: String,
            }
        };

        let fields = generate_field_tokens(&input, ModelKind::Embedded, None)
            .expect("embedded field tokens should generate");

        assert_eq!(fields.setters.len(), 1);

        let setter = compact(&fields.setters[0]);

        assert!(
            setter.contains("pubfn_id<T>"),
            "embedded _id should use an ordinary field setter: \
             {setter}"
        );
        assert!(
            setter.contains("self._id=val.into();"),
            "embedded _id setter should assign the field directly: \
             {setter}"
        );
        assert!(
            !setter.contains("ObjectId"),
            "embedded _id should not use the collection ID setter: \
             {setter}"
        );
    }

    #[test]
    fn field_defaults_override_default_initialization() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[default(String::from("guest"))]
                name: String,

                age: u32,
            }
        };

        let fields = generate_field_tokens(&input, ModelKind::Embedded, None)
            .expect("default expressions should generate");

        assert_eq!(fields.inits.len(), 2);

        let name_init = compact(&fields.inits[0]);
        let age_init = compact(&fields.inits[1]);

        assert!(
            name_init.contains("name:(String::from(\"guest\")).into()"),
            "configured default expression should be used: \
             {name_init}"
        );

        assert!(
            age_init.contains("age:Default::default()"),
            "fields without a configured default should use Default: \
             {age_init}"
        );
    }

    #[test]
    fn collection_fields_route_index_and_validation_attributes() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[index(unique)]
                #[validate(required)]
                name: String,
            }
        };

        let fields = generate_field_tokens(&input, ModelKind::Collection, Some("id"))
            .expect("field attributes should generate");

        assert_eq!(fields.setters.len(), 1);
        assert_eq!(fields.inits.len(), 1);

        assert!(
            !fields.indexes.is_empty(),
            "index attributes should generate index tokens"
        );

        assert!(
            !fields.validations.is_empty(),
            "validation attributes should generate validation tokens"
        );
    }

    #[test]
    fn multiple_indexes_with_distinct_names_are_accepted() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[index(unique, name = "name_idx")]
                name: String,

                #[index(sparse, name = "age_idx")]
                age: i32,

                #[index(text, name = "bio_text_idx")]
                bio: String,
            }
        };

        let fields = generate_field_tokens(&input, ModelKind::Collection, Some("id"))
            .expect("distinct index declarations should generate");

        assert_eq!(fields.indexes.len(), 3);
    }

    #[test]
    fn a_second_text_implying_index_is_rejected() {
        // The second declaration uses a text-associated option (`weight`)
        // without the explicit `text` flag, proving the conflict check uses
        // the same text-implying predicate as index generation.
        let input: DeriveInput = parse_quote! {
            struct Article {
                #[index(text)]
                title: String,

                #[index(weight = 3)]
                body: String,
            }
        };

        let error = match generate_field_tokens(&input, ModelKind::Collection, Some("id")) {
            Ok(_) => panic!("second text-implying index should be rejected"),
            Err(tokens) => tokens.to_string(),
        };

        assert!(
            error.contains("conflicting #[index] declarations")
                && error.contains("`body`")
                && error.contains("`title`"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn duplicate_literal_index_names_are_rejected() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[index(unique, name = "shared_idx")]
                name: String,

                #[index(sparse, name = "shared_idx")]
                age: i32,
            }
        };

        let error = match generate_field_tokens(&input, ModelKind::Collection, Some("id")) {
            Ok(_) => panic!("duplicate index name should be rejected"),
            Err(tokens) => tokens.to_string(),
        };

        assert!(
            error.contains("duplicate #[index] name")
                && error.contains("shared_idx")
                && error.contains("`name`"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn embedded_models_reject_indexes() {
        let input: DeriveInput = parse_quote! {
            struct Address {
                #[index(unique)]
                city: String,
            }
        };

        let error = match generate_field_tokens(&input, ModelKind::Embedded, None) {
            Ok(_) => panic!("embedded index should be rejected"),
            Err(tokens) => tokens.to_string(),
        };

        assert!(
            error.contains("`index` is not supported on embedded models"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn collection_id_requires_a_setter_name() {
        let input: DeriveInput = parse_quote! {
            struct User {
                _id: Option<::oximod::_mongodb::bson::oid::ObjectId>,
            }
        };

        let error = match generate_field_tokens(&input, ModelKind::Collection, None) {
            Ok(_) => {
                panic!("missing collection ID setter should fail")
            }
            Err(tokens) => tokens.to_string(),
        };

        assert!(
            error.contains("collection model is missing its document ID setter name"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn non_struct_models_are_rejected() {
        let input: DeriveInput = parse_quote! {
            enum User {
                Admin,
                Member,
            }
        };

        let error = match generate_field_tokens(&input, ModelKind::Collection, Some("id")) {
            Ok(_) => panic!("enum should be rejected"),
            Err(tokens) => tokens.to_string(),
        };

        assert!(
            error.contains("Model can only be derived for structs."),
            "unexpected error tokens: {error}"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
