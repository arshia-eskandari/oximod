//! Builder-setter generation for collection-model document IDs.
//!
//! Collection models may configure the method name used to assign their
//! MongoDB `_id` field. The generated setter accepts an `ObjectId`, stores it
//! as `Some(id)`, and returns the model for fluent construction.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, parse_str};

/// Appends the configured MongoDB document-ID setter.
///
/// # Errors
///
/// Returns compile-error tokens when `id_setter_name` is not a valid Rust
/// identifier.
pub fn push_id_setter(
    setters: &mut Vec<TokenStream>,
    id_setter_name: &str,
) -> Result<(), TokenStream> {
    let id_method_ident = parse_str::<Ident>(id_setter_name).map_err(|error| {
        syn::Error::new(
            Span::call_site(),
            format!(
                "invalid document ID setter name \
                     `{id_setter_name}`: {error}"
            ),
        )
        .to_compile_error()
    })?;

    let id_setter = quote! {
        /// Sets the MongoDB document ID.
        pub fn #id_method_ident(
            mut self,
            id: ::oximod::_mongodb::bson::oid::ObjectId,
        ) -> Self {
            self._id = Some(id);
            self
        }
    };

    setters.push(id_setter);

    Ok(())
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, quote};

    use super::push_id_setter;

    #[test]
    fn generates_setter_with_configured_name() {
        let mut setters = Vec::new();

        push_id_setter(&mut setters, "with_id").expect("document ID setter should generate");

        assert_eq!(setters.len(), 1);

        let generated = compact(&setters[0]);

        assert!(
            generated.contains("pubfnwith_id("),
            "configured method name should be used; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("id:::oximod::_mongodb::bson::oid::ObjectId"),
            "setter should accept a MongoDB ObjectId; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("self._id=Some(id);"),
            "setter should assign Some(id) to the _id field; \
             generated tokens: {generated}"
        );

        assert!(
            generated.ends_with("self}"),
            "setter should return the model for fluent construction; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn appends_setter_after_existing_tokens() {
        let existing = quote! {
            fn existing() {}
        };

        let mut setters = vec![existing.clone()];

        push_id_setter(&mut setters, "id").expect("document ID setter should generate");

        assert_eq!(setters.len(), 2);
        assert_eq!(compact(&setters[0]), compact(&existing));

        assert!(
            compact(&setters[1]).contains("pubfnid("),
            "new setter should be appended after existing tokens"
        );
    }

    #[test]
    fn invalid_setter_names_return_compile_error_tokens() {
        let mut setters = Vec::new();

        let error = push_id_setter(&mut setters, "with-id")
            .expect_err("invalid setter name should fail")
            .to_string();

        assert!(
            error.contains("invalid document ID setter name `with-id`"),
            "unexpected error tokens: {error}"
        );

        assert!(
            setters.is_empty(),
            "invalid setter names should not append tokens"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
