use crate::default::{push_field_setters, push_id_setter};
use crate::index::generate_index_model_tokens;
use crate::parsers::{parse_attr_value, parse_default_expr, parse_index_args, parse_validate_args};
use crate::validate::generate_validate_model_tokens;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{fmt::Display, str::FromStr};
use syn::{spanned::Spanned, Attribute, DeriveInput, Ident, Type};

/// Generates a compile error token stream for a missing required attribute,
/// using the first attribute's span if available or the call site otherwise.
fn missing_attr_ts(attrs: &[Attribute], msg: &str) -> TokenStream {
    if let Some(first) = attrs.first() {
        syn::Error::new(first.span(), msg).to_compile_error()
    } else {
        syn::Error::new(Span::call_site(), msg).to_compile_error()
    }
}

/// Parses a single-value attribute into type `T` and converts any parse errors
/// into a compile error token stream.
fn parse_attr_value_ts<T>(attr: &Attribute, msg: Option<&str>) -> Result<T, TokenStream>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    parse_attr_value::<T>(attr, msg).map_err(|e| e.to_compile_error())
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

/// Extracts field identifiers and types, processes supported field-level
/// attributes (e.g., `#[index]`, `#[validate]`, `#[default]`), and generates
/// associated token streams for indexes, validations, and initializations.
pub fn collect_field_info(
    input: &DeriveInput,
    all_fields: &mut Vec<(Ident, Type)>,
    has_id_attr: &mut bool,
    indexes: &mut Vec<TokenStream>,
    validations: &mut Vec<TokenStream>,
    inits: &mut Vec<TokenStream>,
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
            all_fields.push((ident.clone(), field.ty.clone()));
            let mut init_expr: TokenStream = quote! { Default::default() };

            for attr in &field.attrs {
                if ident == "_id" {
                    *has_id_attr = true;
                }

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
                    let validation_tokens =
                        generate_validate_model_tokens(ident, &field.ty, validate_args);
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

/// Generates setter method token streams for all fields, including a document
/// ID setter if required.
pub fn setup_setters(
    has_id_attr: bool,
    all_fields: &[(Ident, Type)],
    setters: &mut Vec<TokenStream>,
    document_id_setter_ident: String,
) -> Result<(), TokenStream> {
    push_id_setter(has_id_attr, setters, document_id_setter_ident)?;
    push_field_setters(all_fields, setters);
    Ok(())
}
