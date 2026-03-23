use proc_macro2::TokenStream;
use quote::quote;

/// Appends a setter method for the `_id` field if it exists, using the provided
/// setter name and accepting a MongoDB `ObjectId`.
pub fn push_id_setter(
    setters: &mut Vec<TokenStream>,
    id_setter_name: &str,
) -> Result<(), TokenStream> {
    let id_method_ident = syn::Ident::new(id_setter_name, proc_macro2::Span::call_site());
    let id_setter = quote! {
        /// Set the MongoDB ObjectId
        pub fn #id_method_ident(mut self, id: ::oximod::_mongodb::bson::oid::ObjectId) -> Self {
            self._id = Some(id);
            self
        }
    };
    setters.push(id_setter);

    Ok(())
}
