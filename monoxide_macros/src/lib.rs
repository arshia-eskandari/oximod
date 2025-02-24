use proc_macro::TokenStream;
use syn::{ parse_macro_input, DeriveInput };
use quote::quote;

#[proc_macro_derive(MyTrait)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    
    let expanded =
    quote! {
        impl ::monoxide_core::feature::model::Model for #name {
            async fn save(&self) -> Result<(), ::monoxide_core::feature::error::MongoClientError> {
                let client = ::monoxide_core::get_global_client()?;

                Ok(())
            }
        }
    };

    expanded.into()
}
