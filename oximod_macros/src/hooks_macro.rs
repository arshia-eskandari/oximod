use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub fn generate_hooks_token(name: &Ident) -> TokenStream {
    quote! {
        #[::oximod::_async_trait::async_trait]
        impl ::oximod::Hooks for #name {}
    }
}
