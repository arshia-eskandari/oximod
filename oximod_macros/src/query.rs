use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Field, Fields, Lit, LitStr, Meta, Token, punctuated::Punctuated,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameRule {
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameRule {
    fn parse(value: &LitStr) -> syn::Result<Self> {
        match value.value().as_str() {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            _ => Err(syn::Error::new_spanned(
                value,
                format!("unsupported serde rename rule `{}`", value.value(),),
            )),
        }
    }

    fn apply(self, field_name: &str) -> String {
        match self {
            Self::LowerCase | Self::SnakeCase => field_name.to_owned(),

            Self::UpperCase => field_name.to_ascii_uppercase(),

            Self::PascalCase => to_pascal_case(field_name),

            Self::CamelCase => to_camel_case(field_name),

            Self::ScreamingSnakeCase => field_name.to_ascii_uppercase(),

            Self::KebabCase => field_name.replace('_', "-"),

            Self::ScreamingKebabCase => field_name.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

fn to_pascal_case(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect()
}

fn to_camel_case(value: &str) -> String {
    let pascal_case = to_pascal_case(value);
    let mut characters = pascal_case.chars();

    let Some(first) = characters.next() else {
        return pascal_case;
    };

    first.to_lowercase().chain(characters).collect()
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();

    let Some(first) = characters.next() else {
        return String::new();
    };

    first.to_uppercase().chain(characters).collect()
}

fn container_rename_all(input: &DeriveInput) -> syn::Result<Option<RenameRule>> {
    for attribute in &input.attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        let metadata =
            attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

        for meta in metadata {
            let Meta::NameValue(name_value) = meta else {
                continue;
            };

            if !name_value.path.is_ident("rename_all") {
                continue;
            }

            let Expr::Lit(expression) = &name_value.value else {
                return Err(syn::Error::new_spanned(
                    &name_value.value,
                    "`serde(rename_all)` must contain a string literal",
                ));
            };

            let Lit::Str(rename_rule) = &expression.lit else {
                return Err(syn::Error::new_spanned(
                    expression,
                    "`serde(rename_all)` must contain a string literal",
                ));
            };

            return RenameRule::parse(rename_rule).map(Some);
        }
    }

    Ok(None)
}

fn serialized_field_name(field: &Field, rename_all: Option<RenameRule>) -> syn::Result<LitStr> {
    let field_ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "typed queries require named fields"))?;

    let rust_field_name = field_ident.to_string().trim_start_matches("r#").to_owned();

    let mut explicit_rename = None;

    for attribute in &field.attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        let metadata =
            attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

        for meta in metadata {
            let Meta::NameValue(name_value) = meta else {
                continue;
            };

            if !name_value.path.is_ident("rename") {
                continue;
            }

            let Expr::Lit(expression) = &name_value.value else {
                return Err(syn::Error::new_spanned(
                    &name_value.value,
                    "`serde(rename)` must contain a string literal",
                ));
            };

            let Lit::Str(rename) = &expression.lit else {
                return Err(syn::Error::new_spanned(
                    expression,
                    "`serde(rename)` must contain a string literal",
                ));
            };

            explicit_rename = Some(rename.value());
        }
    }

    let serialized_name = match explicit_rename {
        Some(name) => name,

        None => match rename_all {
            Some(rule) => rule.apply(&rust_field_name),
            None => rust_field_name,
        },
    };

    Ok(LitStr::new(&serialized_name, field_ident.span()))
}

pub fn generate_query_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    generate_query_tokens_inner(input).map_err(|error| error.to_compile_error())
}

fn generate_query_tokens_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model_ident = &input.ident;
    let visibility = &input.vis;
    let fields_ident = format_ident!("{}Fields", model_ident);

    let rename_all = container_rename_all(input)?;

    let named_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,

            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "typed queries require a struct with named fields",
                ));
            }
        },

        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "typed queries can only be generated for structs",
            ));
        }
    };

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
            let field_ident = field.ident.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(field, "typed queries require named fields")
            })?;

            let serialized_name = serialized_field_name(field, rename_all)?;

            Ok(quote! {
                #field_ident:
                    ::oximod::_query::Field::new(
                        #serialized_name
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
            const fn new() -> Self {
                Self {
                    #(#field_initializers,)*
                }
            }
        }

        impl ::oximod::Queryable for #model_ident {
            type Fields = #fields_ident;

            fn fields() -> Self::Fields {
                #fields_ident::new()
            }
        }
    })
}

pub fn generate_embedded_document_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    generate_embedded_document_tokens_inner(input).map_err(|error| error.to_compile_error())
}

fn generate_embedded_document_tokens_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let document_ident = &input.ident;
    let visibility = &input.vis;
    let fields_ident = format_ident!("{}Fields", document_ident);

    let rename_all = container_rename_all(input)?;

    let named_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,

            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "embedded documents require a struct \
                     with named fields",
                ));
            }
        },

        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "EmbeddedDocument can only be derived \
                 for structs",
            ));
        }
    };

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
            let field_ident = field.ident.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(
                    field,
                    "embedded documents require \
                         named fields",
                )
            })?;

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

        impl ::oximod::EmbeddedDocument
            for #document_ident
        {
            type Fields = #fields_ident;

            fn fields_with_prefix(
                prefix: &str,
            ) -> Self::Fields {
                #fields_ident::with_prefix(prefix)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, Field, parse_quote};

    use super::{RenameRule, container_rename_all, serialized_field_name};

    #[test]
    fn field_name_defaults_to_rust_identifier() {
        let field: Field = parse_quote! {
            display_name: String
        };

        let name = serialized_field_name(&field, None).expect("field name should parse");

        assert_eq!(name.value(), "display_name");
    }

    #[test]
    fn field_name_uses_explicit_serde_rename() {
        let field: Field = parse_quote! {
            #[serde(rename = "displayName")]
            display_name: String
        };

        let name = serialized_field_name(&field, Some(RenameRule::UpperCase))
            .expect("serde rename should parse");

        assert_eq!(name.value(), "displayName");
    }

    #[test]
    fn field_name_uses_container_rename_all() {
        let field: Field = parse_quote! {
            display_name: String
        };

        let name = serialized_field_name(&field, Some(RenameRule::CamelCase))
            .expect("rename_all should apply");

        assert_eq!(name.value(), "displayName");
    }

    #[test]
    fn explicit_field_rename_overrides_rename_all() {
        let field: Field = parse_quote! {
            #[serde(rename = "customName")]
            display_name: String
        };

        let name = serialized_field_name(&field, Some(RenameRule::CamelCase))
            .expect("field rename should override rename_all");

        assert_eq!(name.value(), "customName");
    }

    #[test]
    fn field_name_ignores_other_serde_options() {
        let field: Field = parse_quote! {
            #[serde(
                skip_serializing_if = "Option::is_none",
                default
            )]
            _id: Option<ObjectId>
        };

        let name = serialized_field_name(&field, Some(RenameRule::CamelCase))
            .expect("serde options should parse");

        assert_eq!(name.value(), "id");
    }

    #[test]
    fn container_rename_all_is_parsed() {
        let input: DeriveInput = parse_quote! {
            #[serde(rename_all = "camelCase")]
            struct User {
                display_name: String,
            }
        };

        let rule = container_rename_all(&input).expect("rename_all should parse");

        assert_eq!(rule, Some(RenameRule::CamelCase));
    }

    #[test]
    fn standard_serde_rename_rules_are_applied() {
        let cases = [
            (RenameRule::LowerCase, "display_name"),
            (RenameRule::UpperCase, "DISPLAY_NAME"),
            (RenameRule::PascalCase, "DisplayName"),
            (RenameRule::CamelCase, "displayName"),
            (RenameRule::SnakeCase, "display_name"),
            (RenameRule::ScreamingSnakeCase, "DISPLAY_NAME"),
            (RenameRule::KebabCase, "display-name"),
            (RenameRule::ScreamingKebabCase, "DISPLAY-NAME"),
        ];

        for (rule, expected) in cases {
            assert_eq!(rule.apply("display_name"), expected,);
        }
    }
}
