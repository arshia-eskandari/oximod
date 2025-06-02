use syn::{ Attribute, Lit };

#[derive(Default, Debug)]
/// Arguments for creating an index on a field in a MongoDB collection.
///
/// This struct is populated from the `#[index(...)]` attribute
/// and specifies the behavior of the index.
///
/// # Fields
///
/// - `unique`: (Optional) Whether the index enforces a unique constraint.
///   - If `true`, MongoDB will reject documents that cause duplicate values for the indexed field.
///   - Default: `false`
///
/// - `sparse`: (Optional) Whether the index skips documents that are missing the field.
///   - If `true`, documents that do not have the indexed field will not be included in the index.
///   - Default: `false`
///
/// - `name`: (Optional) The custom name for the index.
///   - Useful for identifying indexes manually.
///   - If not provided, MongoDB will generate a default name.
///
/// - `background`: (Optional) Whether the index is built in the background.
///   - If `true`, index creation does not block database operations.
///   - Default: `false`
///
/// - `order`: (Optional) The order of the index.
///   - `1` for ascending order, `-1` for descending order.
///   - Default: `1`
///
/// - `expire_after_secs`: (Optional) The time-to-live (TTL) for the index.
///   - If set, documents will be automatically deleted after the specified number of seconds.
///   - If not provided, documents will not automatically expire.
///
/// # Example
///
/// ```rust
/// #[index(unique = true, sparse = true, name = "email_idx", background = true, order = -1)]
/// email: String,
/// ```
///
/// # Notes
/// - These fields **can be combined freely** — for example, you can have an index that is both `unique` and `sparse`.
/// - MongoDB allows combining `unique`, `sparse`, and `background`.
/// - The `name` field is just metadata and does not conflict with others.
///
pub struct IndexArgs {
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub name: Option<String>,
    pub background: Option<bool>,
    pub order: Option<i32>,
    pub expire_after_secs: Option<i32>,
}

#[derive(Debug)]
pub struct IndexDefinition {
    pub field_name: String,
    pub args: IndexArgs,
}

pub fn parse_index_args(attr: &Attribute, field_name: String) -> syn::Result<IndexDefinition> {
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
                let order_val = match lit {
                    Lit::Int(lit_int) => lit_int.base10_parse::<i32>()?,
                    Lit::Str(lit_str) =>
                        lit_str
                            .value()
                            .parse::<i32>()
                            .map_err(|e|
                                syn::Error::new(
                                    lit_str.span(),
                                    format!("could not parse order: {}", e)
                                )
                            )?,
                    other => {
                        return Err(
                            syn::Error::new(
                                other.span(),
                                "expected integer literal or string literal for `order`"
                            )
                        );
                    }
                };
                args.order = Some(order_val);
            } else if meta.path.is_ident("expire_after_secs") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.expire_after_secs = Some(lit_int.base10_parse::<i32>()?);
                } else {
                    return Err(
                        syn::Error::new(
                            lit.span(),
                            "expected integer literal for `expire_after_secs`"
                        )
                    );
                }
            }
            Ok(())
        })?;
    }

    Ok(IndexDefinition { field_name, args })
}
