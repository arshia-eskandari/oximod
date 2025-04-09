use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, DeriveInput, LitStr };

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
                return syn::Error
                    ::new_spanned(attr, "Expected #[db(\"db_name\"]")
                    .to_compile_error()
                    .into();
            }
        } else if attr.path().is_ident("collection") {
            if let Ok(val) = attr.parse_args::<LitStr>() {
                collection = Some(val);
            } else {
                return syn::Error
                    ::new_spanned(attr, "Expected #[collection(\"collection_name\"]")
                    .to_compile_error()
                    .into();
            }
        }
    }

    let db = match db {
        Some(val) => val,
        None => {
            return syn::Error
                ::new_spanned(&input, "Missing #[db(\"db_name\"] attribute")
                .to_compile_error()
                .into();
        }
    };

    let collection = match collection {
        Some(val) => val,
        None => {
            return syn::Error
                ::new_spanned(&input, "Missing #[collection(\"collection_name\"] attribute")
                .to_compile_error()
                .into();
        }
    };

    let expanded =
        quote! {
        #[async_trait::async_trait]
        impl ::monoxide_core::feature::model::Model for #name {
            async fn save(&self) -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                let client = ::monoxide_core::feature::conn::client::get_global_client()?;
                let db = client.database(#db);
                let collection = db.collection::<::mongodb::bson::Document>(#collection);
                let document = ::mongodb::bson::to_document(&self).map_err(|e| ::monoxide_core::error::conn_error::MongoDbError::SerializationError(e.to_string()))?;
                collection.insert_one(document).await.map_err(|e| {
                    ::monoxide_core::error::conn_error::MongoDbError::ConnectionError(format!("{}", e))
                })?;
                Ok(())
            }

            async fn update() -> Result<(), ::monoxide_core::error::conn_error::MongoDbError> {
                Ok(())
            }
        }
    };

    expanded.into()
}
