//! MongoDB index-model token generation.
//!
//! This module converts parsed [`IndexArgs`] into an expression that constructs
//! a MongoDB `IndexModel`. It is responsible for:
//!
//! - selecting the field's scalar or specialized index key;
//! - forwarding common `IndexOptions` values;
//! - mapping numeric index versions to MongoDB driver enums;
//! - constructing text-index weights;
//! - constructing the case-insensitive collation preset.
//!
//! Attribute syntax and diagnostics remain in `crate::parsers::index`.

use crate::index::IndexArgs;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Ident;

/// Generates an expression that constructs a MongoDB index for `field_ident`.
///
/// Text-index-specific options imply a text index even when the explicit
/// `text` flag is absent. Otherwise, specialized index keys are selected in
/// the following order:
///
/// 1. hashed;
/// 2. wildcard;
/// 3. 2dsphere;
/// 4. 2d;
/// 5. an ordinary ascending or descending scalar index.
///
/// Options that are absent from [`IndexArgs`] are forwarded as `None`, allowing
/// the MongoDB driver to apply its defaults.
pub(crate) fn generate_index_model_tokens(
    field_ident: &Ident,
    index_args: IndexArgs,
) -> TokenStream {
    let key_entry = generate_key_entry(field_ident, &index_args);

    let unique = option_tokens(index_args.unique);
    let sparse = option_tokens(index_args.sparse);
    let background = option_tokens(index_args.background);
    let hidden = option_tokens(index_args.hidden);
    let bits = option_tokens(index_args.bits);
    let min = option_tokens(index_args.min);
    let max = option_tokens(index_args.max);

    let name = owned_string_option_tokens(index_args.name.as_deref());
    let default_language = owned_string_option_tokens(index_args.default_language.as_deref());
    let language_override = owned_string_option_tokens(index_args.language_override.as_deref());

    let expire_after_secs = match index_args.expire_after_secs {
        Some(secs) => {
            quote! { Some(::std::time::Duration::from_secs(#secs as u64)) }
        }
        None => quote! { None },
    };

    let version = match index_args.version {
        Some(1) => quote! {
            Some(::oximod::_mongodb::options::IndexVersion::V1)
        },
        Some(2) => quote! {
            Some(::oximod::_mongodb::options::IndexVersion::V2)
        },
        Some(version) => quote! {
            Some(::oximod::_mongodb::options::IndexVersion::Custom(#version))
        },
        None => quote! { None },
    };

    let text_index_version = match index_args.text_index_version {
        Some(1) => quote! {
            Some(::oximod::_mongodb::options::TextIndexVersion::V1)
        },
        Some(2) => quote! {
            Some(::oximod::_mongodb::options::TextIndexVersion::V2)
        },
        Some(3) => quote! {
            Some(::oximod::_mongodb::options::TextIndexVersion::V3)
        },
        Some(version) => quote! {
            Some(
                ::oximod::_mongodb::options::TextIndexVersion::Custom(#version)
            )
        },
        None => quote! { None },
    };

    let weights = match index_args.weight {
        Some(weight) => {
            quote! {
                Some(::oximod::_mongodb::bson::doc! {
                    stringify!(#field_ident): #weight
                })
            }
        }
        None => quote! { None },
    };

    let sphere_2d_index_version = match index_args.geo_2dsphere_index_version {
        Some(2) => quote! {
            Some(
                ::oximod::_mongodb::options::Sphere2DIndexVersion::V2
            )
        },
        Some(3) => quote! {
            Some(
                ::oximod::_mongodb::options::Sphere2DIndexVersion::V3
            )
        },
        Some(version) => quote! {
            Some(
                ::oximod::_mongodb::options::Sphere2DIndexVersion::Custom(
                    #version
                )
            )
        },
        None => quote! { None },
    };

    let collation = match index_args.case_insensitive {
        Some(true) => {
            quote! {
                Some(
                    ::oximod::_mongodb::options::Collation::builder()
                        .locale("en".to_string())
                        .strength(
                            ::oximod::_mongodb::options::CollationStrength::Secondary
                        )
                        .build()
                )
            }
        }
        Some(false) | None => quote! { None },
    };

    quote! {
        ::oximod::_mongodb::IndexModel::builder()
            .keys(::oximod::_mongodb::bson::doc! { #key_entry })
            .options(
                ::oximod::_mongodb::options::IndexOptions::builder()
                    .unique(#unique)
                    .sparse(#sparse)
                    .background(#background)
                    .name(#name)
                    .expire_after(#expire_after_secs)
                    .version(#version)
                    .default_language(#default_language)
                    .language_override(#language_override)
                    .text_index_version(#text_index_version)
                    .weights(#weights)
                    .sphere_2d_index_version(#sphere_2d_index_version)
                    .bits(#bits)
                    .min(#min)
                    .max(#max)
                    .collation(#collation)
                    .hidden(#hidden)
                    .build()
            )
            .build()
    }
}

/// Generates the document entry used as the index key.
fn generate_key_entry(field_ident: &Ident, index_args: &IndexArgs) -> TokenStream {
    let is_text = index_args.text == Some(true)
        || index_args.text_index_version.is_some()
        || index_args.default_language.is_some()
        || index_args.language_override.is_some()
        || index_args.weight.is_some();

    if is_text {
        quote! { stringify!(#field_ident): "text" }
    } else if index_args.hashed == Some(true) {
        quote! { stringify!(#field_ident): "hashed" }
    } else if index_args.wildcard == Some(true) {
        quote! {
            ::std::concat!(stringify!(#field_ident), ".$**"): 1
        }
    } else if index_args.geo_2dsphere == Some(true) {
        quote! { stringify!(#field_ident): "2dsphere" }
    } else if index_args.geo_2d == Some(true) {
        quote! { stringify!(#field_ident): "2d" }
    } else {
        let order = index_args.order.unwrap_or(1);
        quote! { stringify!(#field_ident): #order }
    }
}

/// Converts an optional tokenizable value into explicit `Some` or `None` tokens.
fn option_tokens<T>(value: Option<T>) -> TokenStream
where
    T: ToTokens,
{
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

/// Converts an optional borrowed string into an owned generated string.
fn owned_string_option_tokens(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value.to_string()) },
        None => quote! { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    fn generated_tokens(index_args: IndexArgs) -> String {
        let field_ident = Ident::new("field", Span::call_site());

        generate_index_model_tokens(&field_ident, index_args)
            .to_string()
            .replace(' ', "")
    }

    #[test]
    fn default_index_uses_an_ascending_scalar_key() {
        let tokens = generated_tokens(IndexArgs::default());

        assert!(
            tokens.contains("doc!{stringify!(field):1"),
            "generated tokens: {tokens}"
        );
    }

    #[test]
    fn scalar_index_uses_the_configured_order() {
        let tokens = generated_tokens(IndexArgs {
            order: Some(-1),
            ..IndexArgs::default()
        });

        assert!(
            tokens.contains("doc!{stringify!(field):-1"),
            "generated tokens: {tokens}"
        );
    }

    #[test]
    fn specialized_index_types_generate_their_expected_keys() {
        let cases = [
            (
                IndexArgs {
                    hashed: Some(true),
                    ..IndexArgs::default()
                },
                "doc!{stringify!(field):\"hashed\"}",
            ),
            (
                IndexArgs {
                    wildcard: Some(true),
                    ..IndexArgs::default()
                },
                "doc!{::std::concat!(stringify!(field),\".$**\"):1",
            ),
            (
                IndexArgs {
                    geo_2dsphere: Some(true),
                    ..IndexArgs::default()
                },
                "doc!{stringify!(field):\"2dsphere\"}",
            ),
            (
                IndexArgs {
                    geo_2d: Some(true),
                    ..IndexArgs::default()
                },
                "doc!{stringify!(field):\"2d\"}",
            ),
        ];

        for (index_args, expected_key) in cases {
            let tokens = generated_tokens(index_args);

            assert!(
                tokens.contains(expected_key),
                "expected `{expected_key}` in `{tokens}`"
            );
        }
    }

    #[test]
    fn text_specific_options_imply_a_text_index() {
        let tokens = generated_tokens(IndexArgs {
            weight: Some(10),
            ..IndexArgs::default()
        });

        assert!(
            tokens.contains("doc!{stringify!(field):\"text\"}"),
            "generated tokens: {tokens}"
        );
        assert!(
            tokens.contains(".weights(Some(::oximod::_mongodb::bson::doc!"),
            "generated tokens: {tokens}"
        );
    }

    #[test]
    fn index_options_are_forwarded_to_the_driver_builder() {
        let tokens = generated_tokens(IndexArgs {
            unique: Some(true),
            sparse: Some(true),
            name: Some("field_index".to_string()),
            background: Some(true),
            expire_after_secs: Some(60),
            version: Some(2),
            text_index_version: Some(3),
            hidden: Some(true),
            text: Some(true),
            case_insensitive: Some(true),
            default_language: Some("english".to_string()),
            language_override: Some("language".to_string()),
            weight: Some(10),
            bits: Some(26),
            min: Some(-180.0),
            max: Some(180.0),
            geo_2dsphere_index_version: Some(3),
            ..IndexArgs::default()
        });

        for expected in [
            ".unique(Some(true))",
            ".sparse(Some(true))",
            ".background(Some(true))",
            ".name(Some(\"field_index\".to_string()))",
            ".expire_after(Some(::std::time::Duration::from_secs(",
            ".version(Some(::oximod::_mongodb::options::IndexVersion::V2))",
            ".default_language(Some(\"english\".to_string()))",
            ".language_override(Some(\"language\".to_string()))",
            ".text_index_version(Some(::oximod::_mongodb::options::TextIndexVersion::V3))",
            ".weights(Some(::oximod::_mongodb::bson::doc!",
            ".sphere_2d_index_version(Some(::oximod::_mongodb::options::Sphere2DIndexVersion::V3))",
            ".bits(Some(",
            ".min(Some(",
            ".max(Some(",
            ".collation(Some(",
            "CollationStrength::Secondary",
            ".hidden(Some(true))",
        ] {
            assert!(
                tokens.contains(expected),
                "expected `{expected}` in `{tokens}`"
            );
        }
    }

    #[test]
    fn unsupported_numeric_versions_use_custom_variants() {
        let tokens = generated_tokens(IndexArgs {
            version: Some(7),
            text_index_version: Some(8),
            geo_2dsphere_index_version: Some(9),
            ..IndexArgs::default()
        });

        for expected in [
            "IndexVersion::Custom(",
            "TextIndexVersion::Custom(",
            "Sphere2DIndexVersion::Custom(",
        ] {
            assert!(
                tokens.contains(expected),
                "expected `{expected}` in `{tokens}`"
            );
        }
    }

    #[test]
    fn explicitly_disabled_case_insensitivity_emits_no_collation() {
        let tokens = generated_tokens(IndexArgs {
            case_insensitive: Some(false),
            ..IndexArgs::default()
        });

        assert!(
            tokens.contains(".collation(None)"),
            "generated tokens: {tokens}"
        );
    }
}
