use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Field, Fields, Lit, LitStr, Meta, Token, punctuated::Punctuated,
};

fn serialized_field_name(field: &Field) -> syn::Result<LitStr> {
    let field_ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "typed queries require named fields"))?;

    let mut serialized_name = field_ident.to_string();

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

            let Expr::Lit(expression) = name_value.value else {
                return Err(syn::Error::new_spanned(
                    name_value,
                    "`serde(rename)` must contain a string literal",
                ));
            };

            let Lit::Str(rename) = expression.lit else {
                return Err(syn::Error::new_spanned(
                    expression,
                    "`serde(rename)` must contain a string literal",
                ));
            };

            serialized_name = rename.value();
        }
    }

    Ok(LitStr::new(&serialized_name, field_ident.span()))
}

pub fn generate_query_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    generate_query_tokens_inner(input).map_err(|error| error.to_compile_error())
}

fn generate_query_tokens_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model_ident = &input.ident;
    let visibility = &input.vis;
    let fields_ident = format_ident!("{}Fields", model_ident);

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
            pub #field_ident: ::oximod::_query::Field<#field_type>
        }
    });

    let field_initializers = named_fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(field, "typed queries require named fields")
            })?;

            let serialized_name = serialized_field_name(field)?;

            Ok(quote! {
                #field_ident: ::oximod::_query::Field::new(
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

#[cfg(test)]
mod tests {
    use syn::{Field, parse_quote};

    use super::serialized_field_name;

    #[test]
    fn field_name_defaults_to_rust_identifier() {
        let field: Field = parse_quote! {
            name: String
        };

        let name = serialized_field_name(&field).expect("field name should parse");

        assert_eq!(name.value(), "name");
    }

    #[test]
    fn field_name_uses_serde_rename() {
        let field: Field = parse_quote! {
            #[serde(rename = "displayName")]
            name: String
        };

        let name = serialized_field_name(&field).expect("serde rename should parse");

        assert_eq!(name.value(), "displayName");
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

        let name = serialized_field_name(&field).expect("serde options should parse");

        assert_eq!(name.value(), "_id");
    }
}
