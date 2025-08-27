use crate::default::{push_field_setter, push_id_setter};
use crate::index::generate_index_model_tokens;
use crate::parsers::{
    parse_attr_value_ts, parse_default_expr, parse_index_args, parse_validate_args,
};
use crate::validate::generate_validate_model_tokens;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Attribute, DeriveInput};

/// Generates a compile error token stream for a missing required attribute,
/// using the first attribute's span if available or the call site otherwise.
fn missing_attr_ts(attrs: &[Attribute], msg: &str) -> TokenStream {
    if let Some(first) = attrs.first() {
        syn::Error::new(first.span(), msg).to_compile_error()
    } else {
        syn::Error::new(Span::call_site(), msg).to_compile_error()
    }
}

/// Collects and validates top-level model attributes, returning core model
/// configuration values or compile error token streams on failure.
pub fn collect_model_attrs(
    attrs: &[Attribute],
) -> Result<(String, String, u32, u8, String), TokenStream> {
    let mut collection: Option<String> = None;
    let mut db: Option<String> = None;
    let mut index_max_retries: u32 = 3;
    let mut index_max_init_seconds: u8 = 30;
    let mut document_id_setter_ident: String = "id".to_string();

    for attr in attrs {
        let path = attr.path();
        if path.is_ident("collection") {
            collection = Some(parse_attr_value_ts::<String>(
                attr,
                Some("Invalid `collection` attribute"),
            )?);
        } else if path.is_ident("db") {
            db = Some(parse_attr_value_ts::<String>(
                attr,
                Some("Invalid `db` attribute"),
            )?);
        } else if path.is_ident("index_max_retries") {
            index_max_retries =
                parse_attr_value_ts::<u32>(attr, Some("Invalid `index_max_retries` attribute"))?;
        } else if path.is_ident("index_max_init_seconds") {
            index_max_init_seconds = parse_attr_value_ts::<u8>(
                attr,
                Some("Invalid `index_max_init_seconds` attribute"),
            )?;
        } else if path.is_ident("document_id_setter_ident") {
            document_id_setter_ident = parse_attr_value_ts::<String>(
                attr,
                Some("Invalid `document_id_setter_ident` attribute"),
            )?;
        }
    }

    let collection = collection.ok_or_else(|| {
        missing_attr_ts(
            attrs,
            r#"Missing #[collection("collection_name")] attribute"#,
        )
    })?;

    let db = db.ok_or_else(|| missing_attr_ts(attrs, r#"Missing #[db("db_name")] attribute"#))?;

    Ok((
        collection,
        db,
        index_max_retries,
        index_max_init_seconds,
        document_id_setter_ident,
    ))
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
    indexes: &mut Vec<TokenStream>,
    validations: &mut Vec<TokenStream>,
    inits: &mut Vec<TokenStream>,
    setters: &mut Vec<TokenStream>,
    document_id_setter_ident: &str,
) -> Result<(), TokenStream> {
    let data_struct = match &input.data {
        syn::Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Model can only be derived for structs.",
            )
            .to_compile_error())
        }
    };

    for field in data_struct.fields.iter() {
        if let Some(ident) = &field.ident {
            if ident == "_id" {
                push_id_setter(setters, document_id_setter_ident)?;
            } else {
                push_field_setter(ident, &field.ty, setters);
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

    Ok(())
}
