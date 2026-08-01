use proc_macro2::TokenStream;
use syn::{Attribute, Ident};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Collection,
    Embedded,
}

impl ModelKind {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self, TokenStream> {
        let mut kind = None;

        for attr in attrs {
            if !attr.path().is_ident("model") {
                continue;
            }

            if kind.is_some() {
                return Err(
                    syn::Error::new_spanned(attr, "duplicate `model` attribute").to_compile_error()
                );
            }

            let option = attr.parse_args::<Ident>().map_err(|error| {
                syn::Error::new_spanned(attr, format!("invalid `model` attribute: {error}"))
                    .to_compile_error()
            })?;

            let parsed_kind = if option == "embedded" {
                Self::Embedded
            } else {
                return Err(syn::Error::new_spanned(
                    option,
                    "unsupported model option; expected `embedded`",
                )
                .to_compile_error());
            };

            kind = Some(parsed_kind);
        }

        Ok(kind.unwrap_or(Self::Collection))
    }
}
