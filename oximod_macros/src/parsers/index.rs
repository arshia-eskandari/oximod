use crate::index::IndexArgs;
use crate::parsers::macros::parse_lit_to_primitive_type;
use syn::{Attribute, Lit};

/// Parses the key-value pairs and flags provided to an `#[index(...)]` attribute into an `IndexArgs` struct.
pub fn parse_index_args(attr: &Attribute) -> syn::Result<IndexArgs> {
    let mut args = IndexArgs::default();

    if attr.path().is_ident("index") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("unique") {
                args.unique = Some(true);
            } else if meta.path.is_ident("sparse") {
                args.sparse = Some(true);
            } else if meta.path.is_ident("background") {
                args.background = Some(true);
            } else if meta.path.is_ident("name") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.name = Some(lit_str.value());
                }
            } else if meta.path.is_ident("order") {
                let lit: Lit = meta.value()?.parse()?;
                let order_val: i32 = parse_lit_to_primitive_type!(lit, i32)?;
                args.order = Some(order_val);
            } else if meta.path.is_ident("expire_after_secs") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.expire_after_secs = Some(lit_int.base10_parse::<i32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `expire_after_secs`",
                    ));
                }
            } else if meta.path.is_ident("version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_primitive_type!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`version` must be greater than or equal to 1",
                    ));
                }
                args.version = Some(version);
            } else if meta.path.is_ident("text_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let text_index_version = parse_lit_to_primitive_type!(&lit, u32)?;
                if text_index_version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`text_index_version` must be greater than or equal to 1",
                    ));
                }
                args.text_index_version = Some(text_index_version);
            } else if meta.path.is_ident("hidden") {
                args.hidden = Some(true);
            } else if meta.path.is_ident("text") {
                args.text = Some(true);
            } else if meta.path.is_ident("hashed") {
                args.hashed = Some(true);
            } else if meta.path.is_ident("wildcard") {
                args.wildcard = Some(true);
            } else if meta.path.is_ident("case_insensitive") {
                args.case_insensitive = Some(true);
            } else if meta.path.is_ident("default_language") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.default_language = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `default_language`",
                    ));
                }
            } else if meta.path.is_ident("language_override") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.language_override = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `language_override`",
                    ));
                }
            } else if meta.path.is_ident("weight") {
                let lit: Lit = meta.value()?.parse()?;
                let weight = parse_lit_to_primitive_type!(&lit, u32)?;
                args.weight = Some(weight);
            } else if meta.path.is_ident("geo_2dsphere") {
                args.geo_2dsphere = Some(true);
            } else if meta.path.is_ident("geo_2d") {
                args.geo_2d = Some(true);
            } else if meta.path.is_ident("bits") {
                let lit: Lit = meta.value()?.parse()?;
                let bits = parse_lit_to_primitive_type!(&lit, u32)?;
                args.bits = Some(bits);
            } else if meta.path.is_ident("min") {
                let lit: Lit = meta.value()?.parse()?;
                let min = parse_lit_to_primitive_type!(&lit, f64)?;
                args.min = Some(min);
            } else if meta.path.is_ident("max") {
                let lit: Lit = meta.value()?.parse()?;
                let max = parse_lit_to_primitive_type!(&lit, f64)?;
                args.max = Some(max);
            } else if meta.path.is_ident("geo_2dsphere_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_primitive_type!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`geo_2dsphere_index_version` must be greater than or equal to 1",
                    ));
                }
                args.geo_2dsphere_index_version = Some(version);
            }

            Ok(())
        })?;
    }

    Ok(args)
}
