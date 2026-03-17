use crate::parsers::literals::{litint_strictly_positive, parse_num_lit_expr};
use crate::validate::ValidateArgs;
use syn::{Attribute, Expr, Lit, Path, parenthesized};

/// Parses the key-value pairs and flags provided to a `#[validate(...)]` attribute into a `ValidateArgs` struc.
pub fn parse_validate_args(attr: &Attribute) -> syn::Result<ValidateArgs> {
    let mut args = ValidateArgs::default();

    if attr.path().is_ident("validate") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("min_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `min_length`",
                    ));
                }
            } else if meta.path.is_ident("max_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `max_length`",
                    ));
                }
            } else if meta.path.is_ident("required") {
                args.required = true;
            } else if meta.path.is_ident("email") {
                args.email = true;
            } else if meta.path.is_ident("pattern") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.pattern = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `pattern`",
                    ));
                }
            } else if meta.path.is_ident("non_empty") {
                args.non_empty = true;
            } else if meta.path.is_ident("positive") {
                args.positive = true;
            } else if meta.path.is_ident("negative") {
                args.negative = true;
            } else if meta.path.is_ident("non_negative") {
                args.non_negative = true;
            } else if meta.path.is_ident("non_positive") {
                args.non_positive = true;
            } else if meta.path.is_ident("min") {
                let expr: Expr = meta.value()?.parse()?;
                args.min = Some(parse_num_lit_expr(expr)?);
            } else if meta.path.is_ident("max") {
                let expr: Expr = meta.value()?.parse()?;
                args.max = Some(parse_num_lit_expr(expr)?);
            } else if meta.path.is_ident("min_exclusive") {
                args.min_exclusive = true;
            } else if meta.path.is_ident("max_exclusive") {
                args.max_exclusive = true;
            } else if meta.path.is_ident("starts_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.starts_with = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `starts_with`",
                    ));
                }
            } else if meta.path.is_ident("ends_with") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.ends_with = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `ends_with`",
                    ));
                }
            } else if meta.path.is_ident("includes") {
                let lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.includes = Some(lit_str.value());
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected string literal for `includes`",
                    ));
                }
            } else if meta.path.is_ident("alphanumeric") {
                args.alphanumeric = true;
            } else if meta.path.is_ident("multiple_of") {
                let lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    litint_strictly_positive(&lit_int)?;
                    args.multiple_of = Some(lit_int);
                } else {
                    return Err(syn::Error::new(
                        lit.span(),
                        "expected integer literal for `multiple_of`",
                    ));
                }
            } else if meta.path.is_ident("custom") {
                if args.custom.is_some() {
                    return Err(meta.error("duplicate `custom` validator"));
                }

                let content;
                parenthesized!(content in meta.input);

                let path: Path = content.parse()?;

                if !content.is_empty() {
                    return Err(meta.error("`custom` accepts exactly one function path"));
                }

                args.custom = Some(path);
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(args)
}
