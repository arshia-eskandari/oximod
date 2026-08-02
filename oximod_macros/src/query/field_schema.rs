//! Generation of typed field-schema structures for models.
//!
//! For a model named `User`, this module generates a hidden `UserFields`
//! structure. Each model field is represented by an
//! `::oximod::_query::Field<T>` containing its serialized MongoDB path.
//!
//! Both collection and embedded models receive a field schema. Embedded
//! schemas can be initialized with a parent path, allowing nested query and
//! update expressions to use complete MongoDB field paths.
//!
//! MongoDB field names are resolved using the model's Serde rename
//! attributes.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Token, punctuated::Punctuated};

use super::serde_name::{container_rename_all, serialized_field_name};

type NamedFields = Punctuated<Field, Token![,]>;

/// Generates a model's hidden typed field structure and `FieldSchema`
/// implementation.
///
/// Syntax errors are converted into compile-error tokens so the derive macro
/// can report them at the relevant source location.
pub(super) fn generate_field_schema_tokens(
    input: &DeriveInput,
) -> Result<TokenStream, TokenStream> {
    generate_field_schema_tokens_inner(input).map_err(|error| error.to_compile_error())
}

fn generate_field_schema_tokens_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model_ident = &input.ident;
    let visibility = &input.vis;
    let fields_ident = format_ident!("{}Fields", model_ident);

    let rename_all = container_rename_all(input)?;
    let named_fields = named_model_fields(input)?;

    let field_declarations = named_fields.iter().map(|field| {
        let field_ident = field
            .ident
            .as_ref()
            .expect("named fields must have identifiers");

        let field_type = &field.ty;

        quote! {
            pub #field_ident:
                ::oximod::_query::Field<#field_type>
        }
    });

    let field_initializers = named_fields
        .iter()
        .map(|field| {
            let field_ident = field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new_spanned(field, "models require named fields"))?;

            let serialized_name = serialized_field_name(field, rename_all)?;

            Ok(quote! {
                #field_ident:
                    ::oximod::_query::Field::from_prefixed(
                        prefix,
                        #serialized_name,
                    )
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #[doc(hidden)]
        #visibility struct #fields_ident {
            #(#field_declarations,)*
        }

        impl #fields_ident {
            #[doc(hidden)]
            fn with_prefix(prefix: &str) -> Self {
                Self {
                    #(#field_initializers,)*
                }
            }
        }

        impl ::oximod::_query::FieldSchema for #model_ident {
            type Fields = #fields_ident;

            fn fields_with_prefix(
                prefix: &str,
            ) -> Self::Fields {
                #fields_ident::with_prefix(prefix)
            }
        }
    })
}

/// Returns the named fields belonging to a supported model declaration.
fn named_model_fields(input: &DeriveInput) -> syn::Result<&NamedFields> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(&fields.named),

            _ => Err(syn::Error::new_spanned(
                &input.ident,
                "models require a struct with named fields",
            )),
        },

        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "Model can only be derived for structs",
        )),
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::{DeriveInput, Fields, Item, Visibility, parse_quote, parse2};

    use super::{
        generate_field_schema_tokens, generate_field_schema_tokens_inner, named_model_fields,
    };

    #[test]
    fn generates_typed_field_structure_for_named_model() {
        let input: DeriveInput = parse_quote! {
            pub struct User {
                name: String,
                age: i32,
            }
        };

        let tokens =
            generate_field_schema_tokens_inner(&input).expect("field schema should generate");

        let generated_file: syn::File = parse2(tokens).expect("generated tokens should parse");

        assert_eq!(generated_file.items.len(), 3);

        let Item::Struct(fields_struct) = &generated_file.items[0] else {
            panic!("first generated item should be a struct");
        };

        assert_eq!(fields_struct.ident, "UserFields");

        assert!(matches!(&fields_struct.vis, Visibility::Public(_)));

        let Fields::Named(fields) = &fields_struct.fields else {
            panic!("generated field structure should have named fields");
        };

        let field_names = fields
            .named
            .iter()
            .map(|field| {
                field
                    .ident
                    .as_ref()
                    .expect("generated field should be named")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(field_names, ["name", "age"]);

        let field_types = fields
            .named
            .iter()
            .map(|field| field.ty.to_token_stream().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            field_types,
            [
                ":: oximod :: _query :: Field < String >",
                ":: oximod :: _query :: Field < i32 >",
            ]
        );
    }

    #[test]
    fn generated_fields_use_serde_serialized_names() {
        let input: DeriveInput = parse_quote! {
            #[serde(rename_all = "camelCase")]
            struct User {
                display_name: String,

                #[serde(rename = "legacy_id")]
                id: i32,
            }
        };

        let generated = generate_field_schema_tokens_inner(&input)
            .expect("field schema should generate")
            .to_string();

        assert!(generated.contains("\"displayName\""));
        assert!(generated.contains("\"legacy_id\""));
    }

    #[test]
    fn generated_schema_uses_supplied_field_prefix() {
        let input: DeriveInput = parse_quote! {
            struct Address {
                city: String,
            }
        };

        let generated = generate_field_schema_tokens_inner(&input)
            .expect("field schema should generate")
            .to_string()
            .replace(' ', "");

        assert!(
            generated.contains("::oximod::_query::Field::from_prefixed(prefix,\"city\",)"),
            "generated schema should pass the parent prefix to every field; \
         generated tokens: {generated}"
        );
    }

    #[test]
    fn tuple_structs_are_rejected() {
        let input: DeriveInput = parse_quote! {
            struct User(String);
        };

        let error = match named_model_fields(&input) {
            Ok(_) => panic!("tuple struct should be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "models require a struct with named fields"
        );
    }

    #[test]
    fn unit_structs_are_rejected() {
        let input: DeriveInput = parse_quote! {
            struct User;
        };

        let error = match named_model_fields(&input) {
            Ok(_) => panic!("unit struct should be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "models require a struct with named fields"
        );
    }

    #[test]
    fn enums_are_rejected() {
        let input: DeriveInput = parse_quote! {
            enum User {
                Admin,
                Member,
            }
        };

        let error = match named_model_fields(&input) {
            Ok(_) => panic!("enum should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "Model can only be derived for structs");
    }

    #[test]
    fn public_entry_point_converts_errors_to_compile_error_tokens() {
        let input: DeriveInput = parse_quote! {
            struct User(String);
        };

        let error_tokens =
            generate_field_schema_tokens(&input).expect_err("tuple struct should fail");

        assert!(error_tokens.to_string().contains("compile_error"));

        assert!(
            error_tokens
                .to_string()
                .contains("models require a struct with named fields")
        );
    }
}
