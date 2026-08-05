//! Generation of collection-model `Queryable` implementations.
//!
//!query.rs`, so your passing local branch is the authoritative version for this cleanup. citeturn225033view2 Collection-backed models receive an implementation of
//! `::oximod::Queryable`, allowing typed queries to begin at the document
//! root.
//!
//! Embedded models do not receive this implementation. They receive only a
//! `FieldSchema` implementation so their fields can participate in nested
//! queries and updates.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

/// Generates a collection model's `Queryable` implementation.
///
/// Generation errors are converted into compile-error tokens so they can be
/// emitted by the derive macro.
pub(super) fn generate_query_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    generate_query_tokens_inner(input).map_err(|error| error.to_compile_error())
}

fn generate_query_tokens_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model_ident = &input.ident;
    let fields_ident = format_ident!("{}Fields", model_ident);

    Ok(quote! {
        impl ::oximod::Queryable for #model_ident {
            type Fields = #fields_ident;

            fn fields() -> Self::Fields {
                <
                    Self as ::oximod::_query::FieldSchema
                >::fields_with_prefix("")
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{DeriveInput, ImplItem, Item, parse_quote, parse2};

    use super::generate_query_tokens_inner;

    #[test]
    fn generates_queryable_implementation_for_model() {
        let input: DeriveInput = parse_quote! {
            struct User {
                name: String,
            }
        };

        let tokens =
            generate_query_tokens_inner(&input).expect("Queryable implementation should generate");

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        assert_eq!(generated_file.items.len(), 1);

        let Item::Impl(queryable_impl) = &generated_file.items[0] else {
            panic!("generated item should be an impl");
        };

        let Some((_, trait_path, _)) = &queryable_impl.trait_ else {
            panic!("generated impl should target a trait");
        };

        assert_eq!(compact_tokens(trait_path), "::oximod::Queryable");

        assert_eq!(compact_tokens(queryable_impl.self_ty.as_ref()), "User");
    }

    #[test]
    fn generated_queryable_uses_model_field_structure() {
        let input: DeriveInput = parse_quote! {
            struct User {
                name: String,
            }
        };

        let tokens =
            generate_query_tokens_inner(&input).expect("Queryable implementation should generate");

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        let Item::Impl(queryable_impl) = &generated_file.items[0] else {
            panic!("generated item should be an impl");
        };

        let associated_type = queryable_impl
            .items
            .iter()
            .find_map(|item| match item {
                ImplItem::Type(item_type) => Some(item_type),
                _ => None,
            })
            .expect("Queryable impl should define Fields");

        assert_eq!(associated_type.ident, "Fields");

        assert_eq!(compact_tokens(&associated_type.ty), "UserFields");
    }

    #[test]
    fn generated_queryable_starts_fields_at_document_root() {
        let input: DeriveInput = parse_quote! {
            struct User {
                name: String,
            }
        };

        let tokens =
            generate_query_tokens_inner(&input).expect("Queryable implementation should generate");

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        let Item::Impl(queryable_impl) = &generated_file.items[0] else {
            panic!("generated item should be an impl");
        };

        let fields_method = queryable_impl
            .items
            .iter()
            .find_map(|item| match item {
                ImplItem::Fn(function) if function.sig.ident == "fields" => Some(function),

                _ => None,
            })
            .expect("Queryable impl should define fields()");

        let method_body = compact_tokens(&fields_method.block);

        assert!(
            method_body.contains(
                "<Selfas::oximod::_query::FieldSchema>\
::fields_with_prefix(\"\")"
            ),
            "fields() should initialize the model schema with \
             an empty root prefix; generated body: {method_body}"
        );
    }

    fn compact_tokens(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
