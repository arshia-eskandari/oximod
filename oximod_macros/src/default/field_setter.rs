//! Builder-setter generation for ordinary model fields.
//!
//! Non-optional fields receive setters that accept any value convertible into
//! the field type. `Option<T>` fields receive setters that accept values
//! convertible into `T` and wrap them in `Some`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::parsers::option_inner_type;

/// Appends a fluent builder setter for an ordinary model field.
///
/// `Option<T>` fields accept a value convertible into `T`; all other fields
/// accept a value convertible into the declared field type.
pub fn push_field_setter(ident: &Ident, ty: &Type, setters: &mut Vec<TokenStream>) {
    let setter = if let Some(inner) = option_inner_type(ty) {
        quote! {
            pub fn #ident<T>(
                mut self,
                val: T,
            ) -> Self
            where
                T: Into<#inner>,
            {
                self.#ident = Some(val.into());
                self
            }
        }
    } else {
        quote! {
            pub fn #ident<T>(
                mut self,
                val: T,
            ) -> Self
            where
                T: Into<#ty>,
            {
                self.#ident = val.into();
                self
            }
        }
    };

    setters.push(setter);
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, format_ident, quote};
    use syn::{Type, parse_quote};

    use super::push_field_setter;

    #[test]
    fn non_optional_fields_generate_direct_setters() {
        let ident = format_ident!("name");
        let ty: Type = parse_quote!(String);
        let mut setters = Vec::new();

        push_field_setter(&ident, &ty, &mut setters);

        assert_eq!(setters.len(), 1);

        let generated = compact(&setters[0]);

        assert!(
            generated.contains("pubfnname<T>(mutself,val:T,)->Self"),
            "setter should use the field name; generated tokens: \
     {generated}"
        );

        assert!(
            generated.contains("T:Into<String>"),
            "setter should accept values convertible into the field type; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("self.name=val.into();"),
            "setter should assign the converted value directly; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn optional_fields_generate_inner_type_setters() {
        let ident = format_ident!("nickname");
        let ty: Type = parse_quote!(Option<String>);
        let mut setters = Vec::new();

        push_field_setter(&ident, &ty, &mut setters);

        assert_eq!(setters.len(), 1);

        let generated = compact(&setters[0]);

        assert!(
            generated.contains("T:Into<String>"),
            "optional setter should accept the inner field type; \
             generated tokens: {generated}"
        );

        assert!(
            !generated.contains("Into<Option<String>>"),
            "optional setter should not require an Option value; \
             generated tokens: {generated}"
        );

        assert!(
            generated.contains("self.nickname=Some(val.into());"),
            "optional setter should wrap the converted value in Some; \
             generated tokens: {generated}"
        );
    }

    #[test]
    fn generated_setter_is_appended_to_existing_tokens() {
        let ident = format_ident!("age");
        let ty: Type = parse_quote!(u32);
        let existing = quote! {
            fn existing() {}
        };
        let mut setters = vec![existing.clone()];

        push_field_setter(&ident, &ty, &mut setters);

        assert_eq!(setters.len(), 2);
        assert_eq!(
            compact(&setters[0]),
            compact(&existing),
            "existing setter tokens should remain unchanged"
        );

        assert!(
            compact(&setters[1]).contains("pubfnage<T>"),
            "new setter should be appended after existing tokens"
        );
    }

    fn compact(tokens: &impl ToTokens) -> String {
        tokens.to_token_stream().to_string().replace(' ', "")
    }
}
