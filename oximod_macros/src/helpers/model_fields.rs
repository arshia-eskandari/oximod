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
