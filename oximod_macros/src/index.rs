pub mod args;

pub use args::IndexArgs;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generates a `TokenStream` that constructs a MongoDB `IndexModel` for the given
/// field identifier based on the provided `IndexArgs`.
///
/// The output tokens configure the index's keys and options, including properties
/// such as uniqueness, sparsity, background creation, name, order, TTL, versioning,
/// text index version, and hidden state.
pub fn generate_index_model_tokens(field_ident: &Ident, index_args: IndexArgs) -> TokenStream {
    let is_text = index_args.text_index_version.is_some();

    let key_entry = if is_text {
        // for text indexes, the value must be the string "text"
        quote! { stringify!(#field_ident): "text" }
    } else {
        // for numeric indexes, use the order
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
                    .text_index_version(#text_index_version)
                    .hidden(#hidden)
                    .build()
            )
            .build()
    }
}
