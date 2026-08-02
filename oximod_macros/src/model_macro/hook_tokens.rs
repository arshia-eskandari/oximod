//! Lifecycle-hook token generation for collection models.
//!
//! When `#[hooks]` is enabled, the generated collection-model implementation
//! invokes the corresponding `Hooks` method before or after each persistence
//! operation. When hooks are disabled, each token stream is empty so the
//! generated implementation has no hook-related calls or trait requirements.

use proc_macro2::TokenStream;
use quote::quote;

/// Generated hook invocations used by the collection-model implementation.
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

/// Generates lifecycle-hook invocations for a collection model.
///
/// Every returned stream is empty when hooks are disabled.
pub fn generate_hook_tokens(hooks: bool) -> HookTokens {
    HookTokens {
        pre_save: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::pre_save(self).await?;
            },
        ),
        post_save: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::post_save(self).await?;
            },
        ),
        pre_save_mut: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::pre_save_mut(self).await?;
            },
        ),
        post_save_mut: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::post_save_mut(self).await?;
            },
        ),
        pre_find: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::pre_find(id.clone()).await?;
            },
        ),
        post_find: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::post_find(&result).await?;
            },
        ),
        pre_delete: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::pre_delete(id.clone()).await?;
            },
        ),
        post_delete: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::post_delete(id).await?;
            },
        ),
        pre_update: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::pre_update(
                    id.clone(),
                    &update,
                )
                .await?;
            },
        ),
        post_update: when_enabled(
            hooks,
            quote! {
                <Self as ::oximod::Hooks>::post_update(
                    id,
                    &update,
                )
                .await?;
            },
        ),
    }
}

fn when_enabled(enabled: bool, tokens: TokenStream) -> TokenStream {
    if enabled { tokens } else { TokenStream::new() }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;

    use super::{HookTokens, generate_hook_tokens};

    #[test]
    fn disabled_hooks_generate_empty_token_streams() {
        let hooks = generate_hook_tokens(false);

        for tokens in hook_streams(&hooks) {
            assert!(tokens.is_empty(), "disabled hooks should not generate code");
        }
    }

    #[test]
    fn enabled_save_hooks_generate_expected_calls() {
        let hooks = generate_hook_tokens(true);

        assert_eq!(
            compact(&hooks.pre_save),
            "<Selfas::oximod::Hooks>::pre_save(self).await?;"
        );
        assert_eq!(
            compact(&hooks.post_save),
            "<Selfas::oximod::Hooks>::post_save(self).await?;"
        );
        assert_eq!(
            compact(&hooks.pre_save_mut),
            "<Selfas::oximod::Hooks>::pre_save_mut(self).await?;"
        );
        assert_eq!(
            compact(&hooks.post_save_mut),
            "<Selfas::oximod::Hooks>::post_save_mut(self).await?;"
        );
    }

    #[test]
    fn enabled_query_hooks_preserve_required_arguments() {
        let hooks = generate_hook_tokens(true);

        assert_eq!(
            compact(&hooks.pre_find),
            "<Selfas::oximod::Hooks>::pre_find(id.clone()).await?;"
        );
        assert_eq!(
            compact(&hooks.post_find),
            "<Selfas::oximod::Hooks>::post_find(&result).await?;"
        );
        assert_eq!(
            compact(&hooks.pre_delete),
            "<Selfas::oximod::Hooks>::pre_delete(id.clone()).await?;"
        );
        assert_eq!(
            compact(&hooks.post_delete),
            "<Selfas::oximod::Hooks>::post_delete(id).await?;"
        );
        assert_eq!(
            compact(&hooks.pre_update),
            "<Selfas::oximod::Hooks>::pre_update(id.clone(),&update,).await?;"
        );

        assert_eq!(
            compact(&hooks.post_update),
            "<Selfas::oximod::Hooks>::post_update(id,&update,).await?;"
        );
    }
    fn hook_streams(hooks: &HookTokens) -> [&TokenStream; 10] {
        [
            &hooks.pre_save,
            &hooks.post_save,
            &hooks.pre_save_mut,
            &hooks.post_save_mut,
            &hooks.pre_find,
            &hooks.post_find,
            &hooks.pre_delete,
            &hooks.post_delete,
            &hooks.pre_update,
            &hooks.post_update,
        ]
    }

    fn compact(tokens: &TokenStream) -> String {
        tokens.to_string().replace(' ', "")
    }
}
