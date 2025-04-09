use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, Data, DeriveInput, Fields, LitStr, Type };

fn is_allowed_type(ty: &Type) -> bool {
    let ty_str = (quote! { #ty }).to_string();

    if matches!(ty_str.as_str(), "String" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool") {
        return true;
    }

    if ty_str.starts_with("Option <") {
        let inner = ty_str.trim_start_matches("Option <").trim_end_matches(">").trim();
        return matches!(inner, "String" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool");
    }

    false
}

#[proc_macro_derive(Model, attributes(db, collection))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut db: Option<LitStr> = None;
    let mut collection: Option<LitStr> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("db") {
            if let Ok(val) = attr.parse_args::<LitStr>() {
                db = Some(val);
            } else {
                return syn::Error::new_spanned(attr, "Expected #[db = \"...\"]")
                    .to_compile_error()
                    .into();
            }
        } else if attr.path().is_ident("collection") {
            if let Ok(val) = attr.parse_args::<LitStr>() {
                collection = Some(val);
            } else {
                return syn::Error::new_spanned(attr, "Expected #[collection = \"...\"]")
                    .to_compile_error()
                    .into();
            }
        }
    }

    let db = match db {
        Some(val) => val,
        None => {
            return syn::Error::new_spanned(&input, "Missing #[db = \"...\"] attribute")
                .to_compile_error()
                .into();
        }
    };

    let collection = match collection {
        Some(val) => val,
        None => {
            return syn::Error::new_spanned(&input, "Missing #[collection = \"...\"] attribute")
                .to_compile_error()
                .into();
        }
    };


    let fields = if let Data::Struct(data) = &input.data {
        if let Fields::Named(ref fields_named) = data.fields {
            &fields_named.named
        } else {
            return syn::Error
                ::new_spanned(&input, "Model can only be derived for structs with named fields")
                .to_compile_error()
                .into();
        }
    } else {
        return syn::Error
            ::new_spanned(&input, "Model can only be derived for structs")
            .to_compile_error()
            .into();
    };

    let mut doc_entries = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().unwrap();
        if !is_allowed_type(&field.ty) {
            return syn::Error
                ::new_spanned(
                    &field.ty,
                    "Field type not supported. Allowed types: String, i32, i64, u32, u64, f32, f64, bool, Option<...>"
                )
                .to_compile_error()
                .into();
        }

        let field_name_str = ident.to_string();
        doc_entries.push(quote! { #field_name_str: &self.#ident });
    }

    let expanded =
        quote! {
        #[async_trait::async_trait]
        impl ::monoxide_core::feature::model::Model for #name {
            async fn save(&self) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                let client = ::monoxide_core::feature::conn::client::get_global_client()?;
                let db = client.database(#db);
                let collection = db.collection::<::mongodb::bson::Document>(#collection);
                let document = ::mongodb::bson::doc! {
                    #(#doc_entries),*
                };
                collection.insert_one(document).await.map_err(|e| {
                    ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(format!("{}", e))
                })?;
                Ok(())
            }
        }
    };

    expanded.into()
}
