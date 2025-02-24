use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;
use monoxide_core::Model;

#[proc_macro_derive(MyTrait)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let name = &input.ident;

    let expanded = quote! {
        impl Model for #name {
            async fn save(&self) -> 
        }
    };

    expanded.into()
}
