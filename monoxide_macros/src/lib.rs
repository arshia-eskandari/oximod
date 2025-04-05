use proc_macro::TokenStream;
use syn::{ parse_macro_input, DeriveInput, Type, Data, Fields };
use quote::quote;

fn is_allowed_type(ty: &Type) -> bool {
    let ty_str = (quote! { #ty }).to_string();
    matches!(ty_str.as_str(), "String" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool")
}

#[proc_macro_derive(Model)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

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
                    "Field type not supported. Allowed types are: String, i32, i64, u32, u64, f32, f64, bool"
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
                let db = client.database("default_db");
                let collection = db.collection(stringify!(#name));
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
