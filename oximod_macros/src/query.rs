use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

pub fn generate_query_tokens(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
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
                )
                .to_compile_error());
            }
        },

        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "typed queries can only be generated for structs",
            )
            .to_compile_error());
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

    let field_initializers = named_fields.iter().map(|field| {
        let field_ident = field
            .ident
            .as_ref()
            .expect("named fields must have identifiers");

        quote! {
            #field_ident: ::oximod::_query::Field::new(
                stringify!(#field_ident)
            )
        }
    });

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
