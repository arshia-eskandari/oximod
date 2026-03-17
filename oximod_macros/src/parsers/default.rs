use syn::{Attribute, Expr};

/// Parses and returns the expression provided as the argument to the given attribute.
pub fn parse_default_expr(attr: &Attribute) -> syn::Result<Expr> {
    let expr: Expr = attr.parse_args()?;
    Ok(expr)
}
