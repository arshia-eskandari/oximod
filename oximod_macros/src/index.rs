pub mod args;
pub use args::IndexArgs;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generates a `TokenStream` that constructs a MongoDB `IndexModel` for the given
/// field identifier based on the provided `IndexArgs`.
pub fn generate_index_model_tokens(field_ident: &Ident, index_args: IndexArgs) -> TokenStream {
    let is_text = index_args.text == Some(true)
        || index_args.text_index_version.is_some()
        || index_args.default_language.is_some()
        || index_args.language_override.is_some()
        || index_args.weight.is_some();

    let key_entry = if is_text {
        quote! { stringify!(#field_ident): "text" }
    } else if index_args.hashed == Some(true) {
        quote! { stringify!(#field_ident): "hashed" }
    } else if index_args.wildcard == Some(true) {
        quote! { ::std::concat!(stringify!(#field_ident), ".$**"): 1 }
    } else if index_args.geo_2dsphere == Some(true) {
        quote! { stringify!(#field_ident): "2dsphere" }
    } else if index_args.geo_2d == Some(true) {
        quote! { stringify!(#field_ident): "2d" }
    } else {
        let order = index_args.order.unwrap_or(1);
        quote! { stringify!(#field_ident): #order }
    };

    let unique = match index_args.unique {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let sparse = match index_args.sparse {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let background = match index_args.background {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let name = match &index_args.name {
        Some(val) => quote! { Some(#val.to_string()) },
        None => quote! { None },
    };

    let expire_after_secs = match index_args.expire_after_secs {
        Some(secs) => quote! { Some(::std::time::Duration::from_secs(#secs as u64)) },
        None => quote! { None },
    };

    let version = match index_args.version {
        Some(1) => quote! { Some(::oximod::_mongodb::options::IndexVersion::V1) },
        Some(2) => quote! { Some(::oximod::_mongodb::options::IndexVersion::V2) },
        Some(v) => quote! { Some(::oximod::_mongodb::options::IndexVersion::Custom(#v)) },
        None => quote! { None },
    };

    let text_index_version = match index_args.text_index_version {
        Some(1) => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V1) },
        Some(2) => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V2) },
        Some(3) => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V3) },
        Some(v) => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::Custom(#v)) },
        None => quote! { None },
    };

    let hidden = match index_args.hidden {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let default_language = match &index_args.default_language {
        Some(val) => quote! { Some(#val.to_string()) },
        None => quote! { None },
    };

    let language_override = match &index_args.language_override {
        Some(val) => quote! { Some(#val.to_string()) },
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
        Some(2) => quote! { Some(::oximod::_mongodb::options::Sphere2DIndexVersion::V2) },
        Some(3) => quote! { Some(::oximod::_mongodb::options::Sphere2DIndexVersion::V3) },
        Some(v) => quote! { Some(::oximod::_mongodb::options::Sphere2DIndexVersion::Custom(#v)) },
        None => quote! { None },
    };

    let bits = match index_args.bits {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let min = match index_args.min {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let max = match index_args.max {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let collation = match index_args.case_insensitive {
        Some(true) => {
            quote! {
                Some(
                    ::oximod::_mongodb::options::Collation::builder()
                        .locale("en".to_string())
                        .strength(::oximod::_mongodb::options::CollationStrength::Secondary)
                        .build()
                )
            }
        }
        _ => quote! { None },
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
