use crate::default::{push_field_setter, push_id_setter};
use crate::index::generate_index_model_tokens;
use crate::parsers::{parse_default_expr, parse_index_args, parse_validate_args};
use crate::validate::generate_validate_model_tokens;
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
/// Specifically:
/// - Inserts setter functions for each field (special-casing `_id`).
/// - Expands `#[index(...)]` attributes into index model tokens.
/// - Expands `#[validate(...)]` attributes into validation tokens.
/// - Expands `#[default = "..."]` attributes into custom initialization
///   expressions (falling back to `Default::default()` otherwise).
/// - Collects all generated tokens into the provided vectors for
///   setters, indexes, validations, and initializations.
///
/// # Errors
/// Returns a `TokenStream` error if the macro target is not a struct
/// or if any attribute arguments fail to parse.
pub fn generate_field_tokens(
    input: &DeriveInput,
    document_id_setter_ident: &str,
) -> Result<FieldTokenStreams, TokenStream> {
    let mut indexes = Vec::new();
    let mut validations = Vec::new();
    let mut inits = Vec::new();
    let mut setters = Vec::new();
    let data_struct = match &input.data {
        syn::Data::Struct(s) => s,
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
            if ident == "_id" {
                push_id_setter(&mut setters, document_id_setter_ident)?;
            } else {
                push_field_setter(ident, &field.ty, &mut setters);
            }

            let mut init_expr: TokenStream = quote! { Default::default() };

            for attr in &field.attrs {
                if attr.path().is_ident("index") {
                    let index_args = parse_index_args(attr).map_err(|err| {
                        syn::Error::new_spanned(attr, format!("Invalid #[index]: {err}"))
                            .to_compile_error()
                    })?;
                    let index_token = generate_index_model_tokens(ident, index_args);
                    indexes.push(index_token);
                } else if attr.path().is_ident("validate") {
                    let validate_args = parse_validate_args(attr).map_err(|err| {
                        syn::Error::new_spanned(attr, format!("Invalid #[validate]: {err}"))
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
                    let default_expr = parse_default_expr(attr).map_err(|err| {
                        syn::Error::new_spanned(attr, format!("Invalid #[default]: {err}"))
                            .to_compile_error()
                    })?;
                    init_expr = quote! { #default_expr };
                }
            }

            inits.push(quote! { #ident: #init_expr });
        }
    }

    Ok(FieldTokenStreams {
        indexes,
        validations,
        inits,
        setters,
    })
}
