use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse_macro_input, Attribute, DeriveInput, Lit, LitStr };

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
struct IndexArgs {
    pub unique: Option<bool>,
    pub sparse: Option<bool>,
    pub name: Option<String>,
    pub background: Option<bool>,
    pub order: Option<i32>,
    pub expire_after_secs: Option<i32>,
}

#[derive(Debug)]
struct IndexDefinition {
    field_name: String,
    args: IndexArgs,
}

fn parse_index_args(attr: &Attribute, field_name: String) -> syn::Result<IndexDefinition> {
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

#[derive(Default, Debug)]
struct ValidateArgs {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub required: Option<bool>,
    // pub enum_values: Option<Vec<String>>, // use rust's enum instead
    pub email: Option<bool>,
    pub pattern: Option<String>,
    pub non_empty: Option<bool>,
    pub positive: Option<bool>,
    pub negative: Option<bool>,
    pub non_negative: Option<bool>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

struct ValidateDefinition {
    field_name: String,
    args: ValidateArgs,
}

fn parse_validate_args(attr: &Attribute, field_name: String) -> syn::Result<ValidateDefinition> {
    let mut args = ValidateArgs::default();

    if attr.path().is_ident("validate") {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("min_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `min_length`")
                    );
                }
            } else if meta.path.is_ident("max_length") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max_length = Some(lit_int.base10_parse::<u32>()?);
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `max_length`")
                    );
                }
            } else if meta.path.is_ident("required") {
                args.required = Some(true);
            // } else if meta.path.is_ident("enum_values") {
            //     // 1. Grab the parenthesized group
            //     let content;
            //     syn::parenthesized!(content in meta.input);

            //     // 2. Parse a comma-separated list of string literals
            //     let values = content
            //         .parse_terminated(
            //             |buf: &syn::parse::ParseBuffer| buf.parse::<syn::LitStr>(), // note the closure
            //             syn::Token![,]
            //         )?
            //         .into_iter()
            //         .map(|lit_str| lit_str.value())
            //         .collect::<Vec<_>>();

            //     args.enum_values = Some(values);
            } else if meta.path.is_ident("email") {
                args.email = Some(true);
            } else if meta.path.is_ident("pattern") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Str(lit_str) = lit {
                    args.pattern = Some(lit_str.value());
                } else {
                    return Err(
                        syn::Error::new(lit.span(), "expected integer literal for `min_length`")
                    );
                }
            } else if meta.path.is_ident("non_empty") {
                args.non_empty = Some(true);
            } else if meta.path.is_ident("positive") {
                args.positive = Some(true);
            } else if meta.path.is_ident("negative") {
                args.negative = Some(true);
            } else if meta.path.is_ident("non_negative") {
                args.non_negative = Some(true);
            } else if meta.path.is_ident("min") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.min = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(lit.span(), "expected integer literal for `min`"));
                }
            } else if meta.path.is_ident("max") {
                let lit: Lit = meta.value()?.parse()?;
                if let Lit::Int(lit_int) = lit {
                    args.max = Some(lit_int.base10_parse::<i64>()?);
                } else {
                    return Err(syn::Error::new(lit.span(), "expected integer literal for `max`"));
                }
            } else {
                return Err(meta.error("unknown attribute key"));
            }

            Ok(())
        })?;
    }

    Ok(ValidateDefinition { field_name, args })
}

#[proc_macro_derive(Model, attributes(db, collection, index, validate))]
/// Procedural macro to derive the `Model` trait for mongodb schema support.
///
/// This macro enables automatic implementation of the `Model` trait, allowing
/// CRUD operations and schema-based mongodb interaction.
///
/// # Required Attributes
///
/// - `#[db("your_database_name")]`: Specifies the database name.
/// - `#[collection("your_collection_name")]`: Specifies the collection name.
///
/// # Example
///
/// ```ignore
/// #[derive(Model, Serialize, Deserialize, Debug)]
/// #[db("test")]
/// #[collection("users")]
/// pub struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     _id: Option<ObjectId>,
///     name: String,
///     age: i32,
///     active: bool,
/// }
/// ```
///
/// Once derived, you can use methods like `.save()`, `.find()`, `.update_one()`, `.delete()`, etc.,
/// provided by the `Model` trait.
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut db: Option<LitStr> = None;
    let mut collection: Option<LitStr> = None;
    let mut index_definitions = Vec::new();
    let mut validate_definitions = Vec::new();

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

    if let syn::Data::Struct(data_struct) = &input.data {
        for field in data_struct.fields.iter() {
            for attr in &field.attrs {
                if attr.path().is_ident("index") {
                    if let Some(ident) = &field.ident {
                        let field_name = ident.to_string();
                        let index_args = parse_index_args(attr, field_name).expect(
                            "could not parse index args"
                        );

                        index_definitions.push(index_args); // <-- COLLECT
                    }
                } else if attr.path().is_ident("validate") {
                    if let Some(ident) = &field.ident {
                        let field_name = ident.to_string();
                        let validate_definition = parse_validate_args(attr, field_name).expect(
                            "could nhot parse validate args"
                        );

                        validate_definitions.push(validate_definition);
                    }
                }
            }
        }
    }

    let index_models = index_definitions.iter().map(|index_def| {
        let field = &index_def.field_name;
        let order = index_def.args.order.unwrap_or(1);

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
            Some(secs) => {
                quote! { Some(::std::time::Duration::from_secs(#secs as u64)) }
            }
            None => quote! { None },
        };

        quote! {
            ::oximod::_mongodb::IndexModel::builder()
                .keys(::oximod::_mongodb::bson::doc! { #field: #order })
                .options(
                    ::oximod::_mongodb::options::IndexOptions::builder()
                        .unique(#unique)
                        .sparse(#sparse)
                        .background(#background)
                        .name(#name)
                        .expire_after(#expire_after_secs)
                        .build()
                )
                .build()
        }
    });

    let validations = validate_definitions.iter().flat_map(|validate_def| {
        let field_ident = syn::Ident::new(&validate_def.field_name, proc_macro2::Span::call_site());
        let ValidateArgs {
            min_length,
            max_length,
            required,
            // enum_values,
            email,
            pattern,
            non_empty,
            positive,
            negative,
            non_negative,
            min,
            max,
        } = &validate_def.args;

        let mut checks = vec![];

        if let Some(min) = min_length {
            checks.push(
                quote! {
                    if self.#field_ident.len() < #min as usize {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be at least {} characters long", stringify!(#field_ident), #min)
                        ));
                    }
                }
            );
        }

        if let Some(max) = max_length {
            checks.push(
                quote! {
                    if self.#field_ident.len() > #max as usize {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be at most {} characters long", stringify!(#field_ident), #max)
                        ));
                    }
                }
            );
        }

        if let Some(req) = required {
            if *req {
                checks.push(
                    quote! {
                        match self.#field_ident {
                            Some(_) => {},
                            None => {
                                return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                    format!("Field '{}' is required", stringify!(#field_ident))
                                ))
                            },
                            _ => {}
                        }
                    }
                );
            }
        }

        // if let Some(values) = enum_values {
        //     let allowed: Vec<proc_macro2::TokenStream> = values
        //         .iter()
        //         .map(|v| quote! { #v })
        //         .collect();

        //     checks.push(
        //         quote! {
        //             if let Some(ref value) = self.#field_ident {
        //                 if ! [#( #allowed ),*].contains(&value.as_str()) {
        //                     return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
        //                         format!(
        //                             "Field '{}' must be one of: {}",
        //                             stringify!(#field_ident),
        //                             vec![#( #allowed.to_string() ),*].join(", ")
        //                         )
        //                     ));
        //                 }
        //             }
        //         }
        //     );
        // }

        if let Some(is_email) = email {
            if *is_email {
                checks.push(
                    quote! {
                        if let Some(email) = &self.#field_ident {
                            if !email.contains('@') || !email.contains('.') {
                                return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                    format!("Field '{}' must be a valid email address", stringify!(#field_ident))
                                ));
                            }
                        
                            let parts: Vec<&str> = email.split('@').collect();
                            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.') {
                                return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                    format!("Field '{}' must be a valid email address", stringify!(#field_ident))
                                ));
                            }
                        } 
                    }
                );
            }
        }

        if let Some(pattern) = pattern {
            checks.push(
                quote! {
                if let Some(ref value) = self.#field_ident {
                    let regex = ::oximod::_regex::Regex::new(#pattern).map_err(|e| {
                        ::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Invalid regex pattern in validation for '{}': {}", stringify!(#field_ident), e)
                        )
                    })?;
                    if !regex.is_match(value) {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!(
                                "Field '{}' does not match the required pattern",
                                stringify!(#field_ident)
                            )
                        ));
                    }
                }
            }
            );
        }

        if let Some(true) = non_empty {
            checks.push(
                quote! {
                let value = &self.#field_ident;
                if let Some(ref val) = value {
                    if val.trim().is_empty() {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be non-empty", stringify!(#field_ident))
                        ));
                    }
                } else {
                    return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                        format!("Field '{}' is missing but marked as non-empty", stringify!(#field_ident))
                    ));
                }
            }
            );
        }

        if let Some(positive) = positive {
            if *positive {
                checks.push(
                    quote! {
                        if self.#field_ident <= 0 {
                            return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                format!("Field '{}' must be positive", stringify!(#field_ident))
                            ))
                        }
                    }
                );
            }
        }

        if let Some(negative) = negative {
            if *negative {
                checks.push(
                    quote! {
                        if self.#field_ident >= 0 {
                            return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                format!("Field '{}' must be negative", stringify!(#field_ident))
                            ))
                        }
                    }
                );
            }
        }

        if let Some(non_negative) = non_negative {
            if *non_negative {
                checks.push(
                    quote! {
                        if self.#field_ident < 0 {
                            return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                                format!("Field '{}' must be non-negative", stringify!(#field_ident))
                            ))
                        }
                    }
                );
            }
        }

        if let Some(min) = min {
            checks.push(
                quote! {
                    if (self.#field_ident as i64) < #min {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be at least {}", stringify!(#field_ident), #min)
                        ));
                    }
                }
            );
        }

        if let Some(max) = max {
            checks.push(
                quote! {
                    if (self.#field_ident as i64) > #max {
                        return Err(::oximod::_error::oximod_error::OximodError::ValidationError(
                            format!("Field '{}' must be at most {}", stringify!(#field_ident), #max)
                        ));
                    }
                }
            );
        }

        checks
    });

    let expanded =
        quote! {

        impl #name {
            fn validate(&self) -> Result<(), ::oximod::_error::oximod_error::OximodError> {
                #(#validations)*
                Ok(())
            }
            
            async fn _create_indexes(
                collection: &::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>
            ) -> Result<(), ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;
    
                let indexes = vec![
                    #(#index_models),*
                ];
    
                if !indexes.is_empty() {
                    collection.create_indexes(indexes).await.map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::IndexError(e.to_string()),
                            "Failed to create indexes on the collection."
                        )
                    })?;
                }
    
                Ok(())
            }
        }

        #[::oximod::_async_trait::async_trait]
        impl ::oximod::_feature::model::Model for #name {

            fn get_collection() -> Result<
                ::oximod::_mongodb::Collection<::oximod::_mongodb::bson::Document>, 
                ::oximod::_error::oximod_error::OximodError
            > {
                let client = ::oximod::_feature::conn::client::get_global_client()?;
                let db = client.database(#db);
                Ok(db.collection::<::oximod::_mongodb::bson::Document>(#collection))
            }
            
            async fn save(&self) -> Result<::oximod::_mongodb::bson::oid::ObjectId, ::oximod::_error::oximod_error::OximodError> {
                self.validate()?; 
                let collection = Self::get_collection()?;
                Self::_create_indexes(&collection).await?; 
                use ::oximod::_error::printable::Printable;

                let document = ::oximod::_mongodb::bson::to_document(&self).map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                        "Failed to serialize model. Are all field types supported by bson::to_document()?"
                    )
                })?;

                let result = collection.insert_one(document).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                        "Failed to insert document. Check if the mongodb server is reachable and the collection exists."
                    )
                })?;

                match result.inserted_id.as_object_id() {
                    Some(id) => Ok(id),
                    None => Err( ::oximod::_attach_printables!(
                        ::oximod::_error::oximod_error::OximodError::SerializationError("inserted_id is not an ObjectId".to_string()),
                        "Expected inserted_id to be an ObjectId but received something else. This may happen if you're using a custom _id."
                    ))
                }
            }

            async fn update(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .update_many(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to update documents. Check your update operators and filter structure."
                        )
                    })?;

                Ok(result)
            }

            async fn update_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .update_one(filter.into(), update.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to update a document. Make sure your update syntax is valid and the filter matches at least one document."
                        )
                    })?;

                Ok(result)
            }

            async fn delete(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .delete_many(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to delete documents. Ensure your filter is valid and matches the correct documents."
                        )
                    })?;

                Ok(result)
            }

            async fn delete_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .delete_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to delete a single document. Ensure your filter is valid and matches the correct document."
                        )
                    })?;

                Ok(result)
            }

            async fn find(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send
            ) -> Result<Vec<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let mut cursor = collection
                    .find(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to execute find query. Double-check your filter syntax or collection state."
                        )
                    })?;

                let mut results = vec![];

                while let Some(doc) = ::oximod::_futures_util::stream::StreamExt::next(&mut cursor).await {
                    let doc = doc.map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Cursor failed to retrieve a document. This may indicate a deserialization or network error mid-stream."
                        )
                    })?;

                    let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                            "Failed to deserialize document into model. Check field types and optionality."
                        )
                    })?;

                    results.push(parsed);
                }

                Ok(results)
            }

            async fn find_one(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;


                let result = collection
                    .find_one(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to run find_one query. Ensure your filter is structured properly and the collection is accessible."
                        )
                    })?;

                match result {
                    Some(doc) => {
                        let parsed = ::oximod::_mongodb::bson::from_document(doc).map_err(|e| {
                            ::oximod::_attach_printables!(
                                ::oximod::_error::oximod_error::OximodError::SerializationError(e.to_string()),
                                "Could not deserialize document into model. Check for type mismatches or missing #[serde] attributes."
                            )
                        })?;
                        Ok(Some(parsed))
                    }
                    None => Ok(None),
                }
            }

            async fn find_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<Option<Self>, ::oximod::_error::oximod_error::OximodError>
            where
                Self: Sized,
            {
                use ::oximod::_error::printable::Printable;

                Self::find_one(::oximod::_mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to find document by _id. Confirm the ID is valid and the document exists."
                    )
                })
            }

            async fn update_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
                update: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<::oximod::_mongodb::results::UpdateResult, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::update_one(::oximod::_mongodb::bson::doc! { "_id": id }, update).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to update document by _id. Check if the document exists and if your update operators are valid."
                    )
                })
            }

            async fn delete_by_id(
                id: ::oximod::_mongodb::bson::oid::ObjectId,
            ) -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::delete_one(::oximod::_mongodb::bson::doc! { "_id": id }).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                        e,
                        "Failed to delete document by _id. Ensure the ID is correct and that the document exists."
                    )
                })
            }

            async fn count(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<u64, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;

                let count = collection
                    .count_documents(filter.into())
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to count documents. Make sure the filter is well-formed and the collection is accessible."
                        )
                    })?;

                Ok(count)
            }

            async fn exists(
                filter: impl Into<::oximod::_mongodb::bson::Document> + Send,
            ) -> Result<bool, ::oximod::_error::oximod_error::OximodError> {
                use ::oximod::_error::printable::Printable;

                Self::find_one(filter).await
                    .map(|opt| opt.is_some())
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            e,
                            "Failed to check document existence. Make sure your filter is valid and your connection is healthy."
                        )
                    })
            }

            async fn clear() -> Result<::oximod::_mongodb::results::DeleteResult, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;

                let result = collection
                    .delete_many(::oximod::_mongodb::bson::doc! {})
                    .await
                    .map_err(|e| {
                        ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::ConnectionError(e.to_string()),
                            "Failed to clear the collection. Ensure the mongodb connection is valid and the collection is writable."
                        )
                    })?;

                Ok(result)
            }

            async fn aggregate(
                pipeline: impl Into<Vec<::oximod::_mongodb::bson::Document>> + Send
            ) -> Result<::oximod::_mongodb::Cursor<oximod::_mongodb::bson::Document>, ::oximod::_error::oximod_error::OximodError> {
                let collection = Self::get_collection()?;
                use ::oximod::_error::printable::Printable;

                let result = collection.aggregate(pipeline.into()).await.map_err(|e| {
                    ::oximod::_attach_printables!(
                            ::oximod::_error::oximod_error::OximodError::AggregationError(e.to_string()),
                            "Failed to aggregate. Ensure your pipeline is valid and the collection is readable."
                    )
                })?;

                Ok(result)
            }
        }
    };

    expanded.into()
}
