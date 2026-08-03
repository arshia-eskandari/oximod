//! Parsing of field-level MongoDB index attributes.
//!
//! The `#[index(...)]` attribute supports:
//!
//! - general index flags such as `unique`, `sparse`, and `hidden`;
//! - scalar index settings such as `name`, `order`, and TTL;
//! - text, hashed, wildcard, and case-insensitive indexes;
//! - 2d and 2dsphere geospatial index settings.
//!
//! This module only parses the supplied arguments. Compatibility checks
//! between index types and their options are handled by the index-generation
//! layer.

use crate::index::IndexArgs;
use crate::parsers::macros::parse_lit_to_primitive_type;
use syn::{Attribute, Lit};

/// Parses the key-value pairs and flags provided to an `#[index(...)]` attribute into an `IndexArgs` struct.
pub fn parse_index_args(attr: &Attribute) -> syn::Result<IndexArgs> {
    let mut args = IndexArgs::default();

    if attr.path().is_ident("index") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("unique") {
                args.unique = Some(true);
            } else if meta.path.is_ident("sparse") {
                args.sparse = Some(true);
            } else if meta.path.is_ident("background") {
                args.background = Some(true);
            } else if meta.path.is_ident("name") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.name = Some(lit_str.value());
                }
            } else if meta.path.is_ident("order") {
                let lit: Lit = meta.value()?.parse()?;
                let order_val: i32 = parse_lit_to_primitive_type!(lit, i32)?;
                args.order = Some(order_val);
            } else if meta.path.is_ident("expire_after_secs") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.expire_after_secs = Some(lit_int.base10_parse::<i32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `expire_after_secs`",
                    ));
                }
            } else if meta.path.is_ident("version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_primitive_type!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`version` must be greater than or equal to 1",
                    ));
                }
                args.version = Some(version);
            } else if meta.path.is_ident("text_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let text_index_version = parse_lit_to_primitive_type!(&lit, u32)?;
                if text_index_version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`text_index_version` must be greater than or equal to 1",
                    ));
                }
                args.text_index_version = Some(text_index_version);
            } else if meta.path.is_ident("hidden") {
                args.hidden = Some(true);
            } else if meta.path.is_ident("text") {
                args.text = Some(true);
            } else if meta.path.is_ident("hashed") {
                args.hashed = Some(true);
            } else if meta.path.is_ident("wildcard") {
                args.wildcard = Some(true);
            } else if meta.path.is_ident("case_insensitive") {
                args.case_insensitive = Some(true);
            } else if meta.path.is_ident("default_language") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.default_language = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `default_language`",
                    ));
                }
            } else if meta.path.is_ident("language_override") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.language_override = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `language_override`",
                    ));
                }
            } else if meta.path.is_ident("weight") {
                let lit: Lit = meta.value()?.parse()?;
                let weight = parse_lit_to_primitive_type!(&lit, u32)?;
                args.weight = Some(weight);
            } else if meta.path.is_ident("geo_2dsphere") {
                args.geo_2dsphere = Some(true);
            } else if meta.path.is_ident("geo_2d") {
                args.geo_2d = Some(true);
            } else if meta.path.is_ident("bits") {
                let lit: Lit = meta.value()?.parse()?;
                let bits = parse_lit_to_primitive_type!(&lit, u32)?;
                args.bits = Some(bits);
            } else if meta.path.is_ident("min") {
                let lit: Lit = meta.value()?.parse()?;
                let min = parse_lit_to_primitive_type!(&lit, f64)?;
                args.min = Some(min);
            } else if meta.path.is_ident("max") {
                let lit: Lit = meta.value()?.parse()?;
                let max = parse_lit_to_primitive_type!(&lit, f64)?;
                args.max = Some(max);
            } else if meta.path.is_ident("geo_2dsphere_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_primitive_type!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`geo_2dsphere_index_version` must be greater than or equal to 1",
                    ));
                }
                args.geo_2dsphere_index_version = Some(version);
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, parse_quote};

    use super::parse_index_args;

    #[test]
    fn parses_general_index_flags() {
        let attr: Attribute = parse_quote! {
            #[index(
                unique,
                sparse,
                background,
                hidden,
                case_insensitive
            )]
        };

        let args = parse_index_args(&attr).expect("general index flags should parse");

        assert_eq!(args.unique, Some(true));
        assert_eq!(args.sparse, Some(true));
        assert_eq!(args.background, Some(true));
        assert_eq!(args.hidden, Some(true));
        assert_eq!(args.case_insensitive, Some(true));
    }

    #[test]
    fn parses_scalar_index_options() {
        let attr: Attribute = parse_quote! {
            #[index(
                name = "email_idx",
                order = 1,
                expire_after_secs = 60,
                version = 2
            )]
        };

        let args = parse_index_args(&attr).expect("scalar index options should parse");

        assert_eq!(args.name.as_deref(), Some("email_idx"));
        assert_eq!(args.order, Some(1));
        assert_eq!(args.expire_after_secs, Some(60));
        assert_eq!(args.version, Some(2));
    }

    #[test]
    fn parses_text_index_options() {
        let attr: Attribute = parse_quote! {
            #[index(
                text,
                text_index_version = 3,
                default_language = "english",
                language_override = "language",
                weight = 10
            )]
        };

        let args = parse_index_args(&attr).expect("text index options should parse");

        assert_eq!(args.text, Some(true));
        assert_eq!(args.text_index_version, Some(3));
        assert_eq!(args.default_language.as_deref(), Some("english"));
        assert_eq!(args.language_override.as_deref(), Some("language"));
        assert_eq!(args.weight, Some(10));
    }

    #[test]
    fn parses_specialized_index_flags() {
        let attr: Attribute = parse_quote! {
            #[index(
                hashed,
                wildcard,
                geo_2dsphere,
                geo_2d
            )]
        };

        let args = parse_index_args(&attr).expect("specialized index flags should parse");

        assert_eq!(args.hashed, Some(true));
        assert_eq!(args.wildcard, Some(true));
        assert_eq!(args.geo_2dsphere, Some(true));
        assert_eq!(args.geo_2d, Some(true));
    }

    #[test]
    fn parses_geospatial_index_options() {
        let attr: Attribute = parse_quote! {
            #[index(
                bits = 26,
                min = 0.0,
                max = 180.0,
                geo_2dsphere_index_version = 3
            )]
        };

        let args = parse_index_args(&attr).expect("geospatial options should parse");

        assert_eq!(args.bits, Some(26));
        assert_eq!(args.min, Some(0.0));
        assert_eq!(args.max, Some(180.0));
        assert_eq!(args.geo_2dsphere_index_version, Some(3));
    }

    #[test]
    fn version_options_must_be_greater_than_zero() {
        let version_attr: Attribute = parse_quote! {
            #[index(version = 0)]
        };

        let error = parse_index_args(&version_attr).expect_err("zero index version should fail");

        assert_eq!(
            error.to_string(),
            "`version` must be greater than or equal to 1"
        );

        let text_version_attr: Attribute = parse_quote! {
            #[index(text_index_version = 0)]
        };

        let error =
            parse_index_args(&text_version_attr).expect_err("zero text index version should fail");

        assert_eq!(
            error.to_string(),
            "`text_index_version` must be greater than or equal to 1"
        );

        let sphere_version_attr: Attribute = parse_quote! {
            #[index(geo_2dsphere_index_version = 0)]
        };

        let error =
            parse_index_args(&sphere_version_attr).expect_err("zero 2dsphere version should fail");

        assert_eq!(
            error.to_string(),
            "`geo_2dsphere_index_version` must be greater than or equal to 1"
        );
    }

    #[test]
    fn expire_after_seconds_requires_an_integer() {
        let attr: Attribute = parse_quote! {
            #[index(expire_after_secs = "sixty")]
        };

        let error = parse_index_args(&attr).expect_err("non-integer TTL should fail");

        assert_eq!(
            error.to_string(),
            "expected integer literal for `expire_after_secs`"
        );
    }

    #[test]
    fn string_options_require_string_literals() {
        let default_language: Attribute = parse_quote! {
            #[index(default_language = 5)]
        };

        let error = parse_index_args(&default_language).expect_err("numeric language should fail");

        assert_eq!(
            error.to_string(),
            "expected string literal for `default_language`"
        );

        let language_override: Attribute = parse_quote! {
            #[index(language_override = 5)]
        };

        let error = parse_index_args(&language_override)
            .expect_err("numeric language override should fail");

        assert_eq!(
            error.to_string(),
            "expected string literal for `language_override`"
        );
    }

    #[test]
    fn unknown_index_options_are_rejected() {
        let attr: Attribute = parse_quote! {
            #[index(unknown)]
        };

        let error = parse_index_args(&attr).expect_err("unknown index option should fail");

        assert_eq!(error.to_string(), "unknown attribute key");
    }

    #[test]
    fn unrelated_attributes_produce_default_arguments() {
        let attr: Attribute = parse_quote! {
            #[serde(default)]
        };

        let args = parse_index_args(&attr).expect("unrelated attribute should be ignored");

        assert!(args.unique.is_none());
        assert!(args.name.is_none());
        assert!(args.order.is_none());
        assert!(args.text.is_none());
        assert!(args.geo_2dsphere.is_none());
    }
}
