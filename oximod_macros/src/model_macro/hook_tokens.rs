use proc_macro2::TokenStream;
use quote::quote;

pub struct HookTokens {
    pub pre_save: TokenStream,
    pub post_save: TokenStream,
    pub pre_save_mut: TokenStream,
    pub post_save_mut: TokenStream,
    pub pre_find: TokenStream,
    pub post_find: TokenStream,
    pub pre_delete: TokenStream,
    pub post_delete: TokenStream,
    pub pre_update: TokenStream,
    pub post_update: TokenStream,
}

pub fn generate_hook_tokens(hooks: bool) -> HookTokens {
    HookTokens {
        pre_save: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_save(self).await?; }
        } else {
            quote! {}
        },

        post_save: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_save(self).await?; }
        } else {
            quote! {}
        },

        pre_save_mut: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_save_mut(self).await?; }
        } else {
            quote! {}
        },

        post_save_mut: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_save_mut(self).await?; }
        } else {
            quote! {}
        },

        pre_find: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_find(id.clone()).await?; }
        } else {
            quote! {}
        },

        post_find: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_find(&result).await?; }
        } else {
            quote! {}
        },

        pre_delete: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_delete(id.clone()).await?; }
        } else {
            quote! {}
        },

        post_delete: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_delete(id).await?; }
        } else {
            quote! {}
        },

        pre_update: if hooks {
            quote! { <Self as ::oximod::Hooks>::pre_update(id.clone(), &update).await?; }
        } else {
            quote! {}
        },

        post_update: if hooks {
            quote! { <Self as ::oximod::Hooks>::post_update(id, &update).await?; }
        } else {
            quote! {}
        },
    }
}
