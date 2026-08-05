//! Parsing and validation of model-level derive attributes.
//!
//! Collection models require database and collection names and may configure
//! hooks, index initialization metadata, and the generated document-ID setter.
//!
//! Embedded models do not have independent MongoDB collections, so
//! collection-specific attributes are rejected for them.

use crate::{helpers::ModelKind, parsers::parse_attr_value_ts};
use proc_macro2::{Span, TokenStream};
use syn::{Attribute, spanned::Spanned};

/// Generates a compile error token stream for a missing required attribute,
/// using the first attribute's span if available or the call site otherwise.
fn missing_attr_ts(attrs: &[Attribute], msg: &str) -> TokenStream {
    if let Some(first) = attrs.first() {
        syn::Error::new(first.span(), msg).to_compile_error()
    } else {
        syn::Error::new(Span::call_site(), msg).to_compile_error()
    }
}

/// Collection-specific configuration for a collection-backed model.
///
/// These attributes are required or applicable only when deriving a regular
/// collection-backed model. Embedded models do not have an independent MongoDB
/// collection and therefore do not receive this configuration.
pub struct CollectionAttrs {
    pub collection: String,
    pub db: String,
    pub index_max_retries: u32,
    pub index_max_init_seconds: u8,
    pub document_id_setter_ident: String,
    pub hooks: bool,
}

/// Top-level configuration collected for a model.
///
/// Every model has a [`ModelKind`]. Collection-backed models additionally
/// contain [`CollectionAttrs`], while embedded models have no collection
/// configuration.
pub struct ModelAttrs {
    pub kind: ModelKind,
    pub collection: Option<CollectionAttrs>,
}

/// Collects and validates top-level model attributes, returning core model
/// configuration values or compile error token streams on failure.
///
/// Models are collection-backed by default. Collection-backed models require
/// both `#[db(...)]` and `#[collection(...)]`.
///
/// Models marked with `#[model(embedded)]` do not require collection
/// configuration and reject attributes that are only meaningful for
/// collection-backed models.
pub fn collect_model_attrs(attrs: &[Attribute]) -> Result<ModelAttrs, TokenStream> {
    let kind = ModelKind::from_attrs(attrs)?;

    let mut collection: Option<String> = None;
    let mut db: Option<String> = None;
    let mut index_max_retries: u32 = 3;
    let mut index_max_init_seconds: u8 = 30;
    let mut document_id_setter_ident: String = "id".to_string();
    let mut hooks = false;

    for attr in attrs {
        let path = attr.path();

        if path.is_ident("model") {
            // The model mode is parsed separately by `ModelKind::from_attrs`.
            continue;
        }

        if path.is_ident("serde") {
            // Serde container attributes, such as `rename_all`, are handled
            // separately by the generated serialization and query code.
            continue;
        }

        if kind == ModelKind::Embedded {
            let unsupported_attribute = if path.is_ident("collection") {
                Some("collection")
            } else if path.is_ident("db") {
                Some("db")
            } else if path.is_ident("index_max_retries") {
                Some("index_max_retries")
            } else if path.is_ident("index_max_init_seconds") {
                Some("index_max_init_seconds")
            } else if path.is_ident("document_id_setter_ident") {
                Some("document_id_setter_ident")
            } else if path.is_ident("hooks") {
                Some("hooks")
            } else {
                None
            };

            if let Some(attribute_name) = unsupported_attribute {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("`{attribute_name}` is not supported on embedded models"),
                )
                .to_compile_error());
            }

            return Err(syn::Error::new_spanned(
                attr,
                "Unsupported attribute for #[derive(Model)]",
            )
            .to_compile_error());
        }

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
        } else if path.is_ident("hooks") {
            hooks = true;
        } else {
            return Err(syn::Error::new_spanned(
                attr,
                "Unsupported attribute for #[derive(Model)]",
            )
            .to_compile_error());
        }
    }

    if kind == ModelKind::Embedded {
        return Ok(ModelAttrs {
            kind,
            collection: None,
        });
    }

    let collection = collection.ok_or_else(|| {
        missing_attr_ts(
            attrs,
            r#"Missing #[collection("collection_name")] attribute"#,
        )
    })?;

    let db = db.ok_or_else(|| missing_attr_ts(attrs, r#"Missing #[db("db_name")] attribute"#))?;

    Ok(ModelAttrs {
        kind,
        collection: Some(CollectionAttrs {
            collection,
            db,
            index_max_retries,
            index_max_init_seconds,
            document_id_setter_ident,
            hooks,
        }),
    })
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use syn::parse_quote;

    use super::{ModelAttrs, collect_model_attrs};
    use crate::helpers::ModelKind;

    #[test]
    fn collection_attributes_are_collected_with_defaults() {
        let attrs = vec![
            parse_quote! {
                #[db("app_db")]
            },
            parse_quote! {
                #[collection("users")]
            },
        ];

        let model_attrs = collect_model_attrs(&attrs).expect("collection attributes should parse");

        assert_eq!(model_attrs.kind, ModelKind::Collection);

        let collection = model_attrs
            .collection
            .expect("collection model should have configuration");

        assert_eq!(collection.db, "app_db");
        assert_eq!(collection.collection, "users");
        assert_eq!(collection.index_max_retries, 3);
        assert_eq!(collection.index_max_init_seconds, 30);
        assert_eq!(collection.document_id_setter_ident, "id");
        assert!(!collection.hooks);
    }

    #[test]
    fn collection_attributes_honor_explicit_configuration() {
        let attrs = vec![
            parse_quote! {
                #[db("app_db")]
            },
            parse_quote! {
                #[collection("users")]
            },
            parse_quote! {
                #[index_max_retries(5)]
            },
            parse_quote! {
                #[index_max_init_seconds(45)]
            },
            parse_quote! {
                #[document_id_setter_ident("with_id")]
            },
            parse_quote! {
                #[hooks]
            },
        ];

        let model_attrs = collect_model_attrs(&attrs).expect("collection attributes should parse");

        let collection = model_attrs
            .collection
            .expect("collection model should have configuration");

        assert_eq!(collection.index_max_retries, 5);
        assert_eq!(collection.index_max_init_seconds, 45);
        assert_eq!(collection.document_id_setter_ident, "with_id");
        assert!(collection.hooks);
    }

    #[test]
    fn embedded_models_need_no_collection_configuration() {
        let attrs = vec![
            parse_quote! {
                #[model(embedded)]
            },
            parse_quote! {
                #[serde(rename_all = "camelCase")]
            },
        ];

        let model_attrs = collect_model_attrs(&attrs).expect("embedded attributes should parse");

        assert_eq!(model_attrs.kind, ModelKind::Embedded);
        assert!(model_attrs.collection.is_none());
    }

    #[test]
    fn embedded_models_reject_collection_attributes() {
        let attrs = vec![
            parse_quote! {
                #[model(embedded)]
            },
            parse_quote! {
                #[collection("addresses")]
            },
        ];

        let error = error_text(
            collect_model_attrs(&attrs),
            "embedded collection attribute should fail",
        );

        assert!(
            error.contains("`collection` is not supported on embedded models"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn collection_models_require_database_and_collection() {
        let missing_collection = vec![parse_quote! {
            #[db("app_db")]
        }];

        let error = error_text(
            collect_model_attrs(&missing_collection),
            "missing collection should fail",
        );

        assert!(
            error.contains("Missing #[collection(\\\"collection_name\\\")] attribute"),
            "unexpected error tokens: {error}"
        );

        let missing_database = vec![parse_quote! {
            #[collection("users")]
        }];

        let error = error_text(
            collect_model_attrs(&missing_database),
            "missing database should fail",
        );

        assert!(
            error.contains("Missing #[db(\\\"db_name\\\")] attribute"),
            "unexpected error tokens: {error}"
        );
    }

    #[test]
    fn unsupported_model_attributes_are_rejected() {
        let attrs = vec![
            parse_quote! {
                #[db("app_db")]
            },
            parse_quote! {
                #[collection("users")]
            },
            parse_quote! {
                #[unknown]
            },
        ];

        let error = error_text(
            collect_model_attrs(&attrs),
            "unsupported attribute should fail",
        );

        assert!(
            error.contains("Unsupported attribute for #[derive(Model)]"),
            "unexpected error tokens: {error}"
        );
    }

    fn error_text(result: Result<ModelAttrs, TokenStream>, failure_message: &str) -> String {
        match result {
            Ok(_) => panic!("{failure_message}"),
            Err(tokens) => tokens.to_string(),
        }
    }
}
