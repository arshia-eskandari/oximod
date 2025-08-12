use crate::default::{parse_default_expr, push_field_setters, push_id_setter};
use crate::index::{generate_index_model_tokens, parse_index_args};
use crate::validate::{generate_validate_model_tokens, parse_validate_args};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Ident, LitInt, LitStr, Type};

pub fn parse_lit_str(attr: &Attribute, expected_name: &str) -> Result<LitStr, TokenStream> {
    attr.parse_args::<LitStr>().map_err(|_| {
        syn::Error::new_spanned(attr, format!("Expected #[{}(\"...\")]", expected_name))
            .to_compile_error()
    })
}

pub fn parse_lit_u32(attr: &Attribute, expected_name: &str) -> Result<u32, TokenStream> {
    let lit = attr.parse_args::<LitInt>().map_err(|_| {
        syn::Error::new_spanned(attr, format!("Expected #[{}(<u32>)]", expected_name))
            .to_compile_error()
    })?;
    lit.base10_parse::<u32>().map_err(|e| {
        syn::Error::new_spanned(attr, format!("Invalid u32 for {}: {}", expected_name, e))
            .to_compile_error()
    })
}

pub fn parse_lit_u8(attr: &Attribute, expected_name: &str) -> Result<u8, TokenStream> {
    let lit = attr.parse_args::<LitInt>().map_err(|_| {
        syn::Error::new_spanned(attr, format!("Expected #[{}(<u8>)]", expected_name))
            .to_compile_error()
    })?;
    lit.base10_parse::<u8>().map_err(|e| {
        syn::Error::new_spanned(attr, format!("Invalid u8 for {}: {}", expected_name, e))
            .to_compile_error()
    })
}

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
            collection = Some(parse_lit_str(attr, "collection")?.value());
        } else if path.is_ident("db") {
            db = Some(parse_lit_str(attr, "db")?.value());
        } else if path.is_ident("index_max_retries") {
            index_max_retries = parse_lit_u32(attr, "index_max_retries")?;
        } else if path.is_ident("index_max_init_seconds") {
            index_max_init_seconds = parse_lit_u8(attr, "index_max_init_seconds")?;
        } else if path.is_ident("document_id_setter_ident") {
            document_id_setter_ident = parse_lit_str(attr, "document_id_setter_ident")?.value();
        }
    }

    let collection = match collection {
        Some(v) => v,
        None => {
            return Err(syn::Error::new_spanned(
                attrs.get(0).unwrap_or(&Attribute {
                    pound_token: Default::default(),
                    style: syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    meta: syn::parse_quote!(doc = "missing collection"),
                }),
                "Missing #[collection(\"collection_name\")] attribute",
            )
            .to_compile_error());
        }
    };

    let db = match db {
        Some(v) => v,
        None => {
            return Err(syn::Error::new_spanned(
                attrs.get(0).unwrap_or(&Attribute {
                    pound_token: Default::default(),
                    style: syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    meta: syn::parse_quote!(doc = "missing db"),
                }),
                "Missing #[db(\"db_name\")] attribute",
            )
            .to_compile_error())
        }
    };

    Ok((
        collection,
        db,
        index_max_retries,
        index_max_init_seconds,
        document_id_setter_ident,
    ))
}

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

pub fn setup_setters(
    has_id_attr: bool,
    all_fields: &[(Ident, Type)],
    setters: &mut Vec<TokenStream>,
    document_id_setter_ident: String,
) -> Result<(), TokenStream> {
    if let Err(e) = push_id_setter(has_id_attr, setters, document_id_setter_ident) {
        return Err(e);
    }
    push_field_setters(all_fields, setters);
    Ok(())
}
