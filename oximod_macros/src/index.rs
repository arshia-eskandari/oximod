use proc_macro2::TokenStream;
use quote::quote;
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
/// - `version`: (Optional) The version of the index structure to use.
///   - Applies to standard indexes (e.g., B-tree).
///   - Most common values are `1` or `2`; `Custom(N)` can also be specified.
///   - Only meaningful for certain index types; may be ignored for default scalar indexes.
///
/// - `text_index_version`: (Optional) The version of the text index structure to use.
///   - Applies only to `text` indexes.
///   - Supported values include `1`, `2`, and `3`, or `Custom(N)`.
///   - Use this to explicitly control MongoDB's text indexing behavior.
///
/// - `hidden`: (Optional) Whether the index is hidden from the query planner.
///   - If `true`, the index exists but will not be used by the query planner unless explicitly hinted.
///   - Useful for testing or safely rolling out new indexes.
///   - Default: `false`
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
/// - Version fields are **optional** and typically only needed for compatibility or performance tuning.
/// - `text_index_version` is only applicable if the index type is explicitly `text`.
/// - ⚠️ If both `order` and `text_index_version` are provided, `order` will be ignored. 
///
pub struct IndexArgs {
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub name: Option<String>,
    pub background: Option<bool>,
    pub order: Option<i32>,
    pub expire_after_secs: Option<i32>,
    pub version: Option<u32>,
    pub text_index_version: Option<u32>,
    pub hidden: Option<bool>,
}

#[derive(Debug)]
pub struct IndexDefinition {
    pub field_name: String,
    pub args: IndexArgs,
}

macro_rules! parse_lit_to_number {
    ($lit:expr, $ty:ty) => {{
        match $lit {
            syn::Lit::Int(lit_int) => lit_int.base10_parse::<$ty>(),
            syn::Lit::Str(lit_str) => lit_str.value().parse::<$ty>()
                .map_err(|e| syn::Error::new(lit_str.span(), format!("could not parse number: {}", e))),
            other => Err(syn::Error::new(other.span(), "expected integer or string literal")),
        }
    }};
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
                let order_val: i32 = parse_lit_to_number!(lit, i32)?;
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
            } else if meta.path.is_ident("version") {
                let lit: Lit = meta.value()?.parse()?;
                let version = parse_lit_to_number!(&lit, u32)?;
                if version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`text_index_version` must be >= 1",
                    ));
                }
                args.version = Some(version);
            } else if meta.path.is_ident("text_index_version") {
                let lit: Lit = meta.value()?.parse()?;
                let text_index_version = parse_lit_to_number!(&lit, u32)?;
                if text_index_version == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "`text_index_version` must be >= 1",
                    ));
                }
                args.text_index_version = Some(text_index_version); 
            } else if meta.path.is_ident("hidden") {
                args.hidden = Some(true);
            }

            Ok(())
        })?;
    }

    Ok(IndexDefinition { field_name, args })
}

pub fn generate_index_model_tokens(index_def: &IndexDefinition) -> TokenStream {
    let field = &index_def.field_name;
    let is_text = index_def.args.text_index_version.is_some();
    
    let key_entry = if is_text {
        // for text indexes, the value must be the string "text"
        quote! { #field: "text" }
    } else {
        // for numeric indexes, use the order
        let order = index_def.args.order.unwrap_or(1);
        quote! { #field: #order }
    };

    let unique = match index_def.args.unique {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let sparse = match index_def.args.sparse {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let background = match index_def.args.background {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    let name = match &index_def.args.name {
        Some(val) => quote! { Some(#val.to_string()) },
        None => quote! { None },
    };

    let expire_after_secs = match index_def.args.expire_after_secs {
        Some(secs) => quote! { Some(::std::time::Duration::from_secs(#secs as u64)) },
        None => quote! { None },
    };

    let version = match index_def.args.version {
        Some(v) if v == 1 => quote! { Some(::oximod::_mongodb::options::IndexVersion::V1) },
        Some(v) if v == 2 => quote! { Some(::oximod::_mongodb::options::IndexVersion::V2) },
        Some(v) => quote! { Some(::oximod::_mongodb::options::IndexVersion::Custom(#v)) },
        None => quote! { None },
    };
    
    let text_index_version = match index_def.args.text_index_version {
        Some(v) if v == 1 => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V1) },
        Some(v) if v == 2 => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V2) },
        Some(v) if v == 3 => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::V3) },
        Some(v) => quote! { Some(::oximod::_mongodb::options::TextIndexVersion::Custom(#v)) },
        None => quote! { None },
    };

    let hidden = match index_def.args.hidden {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    quote! {
        ::oximod::_mongodb::IndexModel::builder()
            .keys(::oximod::_mongodb::bson::doc! { #key_entry })
            .options(
                ::oximod::_mongodb::options::IndexOptions::builder()
                    .unique(#unique)
                    .sparse(#sparse)
                    .background(#background)
                    .name(#name)
                    .expire_after(#expire_after_secs)
                    .version(#version)
                    .text_index_version(#text_index_version)
                    .hidden(#hidden)
                    .build()
            )
            .build()
    }
}
