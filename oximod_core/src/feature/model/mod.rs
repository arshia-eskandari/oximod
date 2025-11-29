use crate::error::oximod_error::OxiModError;
use async_trait;
use mongodb::{
    bson::{self, oid::ObjectId, Document},
    results::{DeleteResult, UpdateResult},
    Collection,
};

/// An asynchronous trait for MongoDB models enabling CRUD operations, typically implemented via the #[derive(Model)] macro.
#[async_trait::async_trait]
pub trait Model {
    /// Retrieves the MongoDB collection associated with the model using the passed in client.
    ///
    /// This method is typically used internally by the framework, but it can be called
    /// directly when you need low-level access to the collection—such as for creating
    /// indexes manually or performing custom MongoDB operations not covered by the trait.
    ///
    /// # Returns
    /// - [`Collection<Document>`](https://docs.rs/mongodb/latest/mongodb/struct.Collection.html): A handle to the MongoDB collection.
    /// - [`OxiModError`](crate::error::oximod_error::OximodError): If the global client is not initialized or the collection name is missing.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let collection = User::get_collection_with_client(client)?;
    /// let count = collection.count_documents(doc! {}).await?;
    /// println!("Total documents: {}", count);
    /// ```
    fn get_collection_with_client(
        client: &mongodb::Client,
    ) -> Result<Collection<Document>, OxiModError>;
    /// Inserts the current model instance into the MongoDB collection using the passed in client.
    ///
    /// # Returns
    /// - `ObjectId` of the inserted document.
    ///
    /// # Example
    /// ```rust,ignore
    ///
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let id = user.save_with_client(client).await?;
    /// println!("Inserted user ID: {}", id);
    /// ```
    async fn save_with_client(&self, client: &mongodb::Client) -> Result<ObjectId, OxiModError>;
    /// Updates all documents in the collection that match the given filter using the passed in
    /// client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document specifying which documents to match.
    /// - `update`: A BSON document with the update operations to apply.
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) containing matched and modified counts.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let result = User::update_with_client(doc! { "active": false }, doc! { "$set": { "active": true } }, client).await?;
    /// assert_eq!(result.modified_count, 3);
    /// ```
    async fn update_with_client(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<UpdateResult, OxiModError>;
    /// Updates the **first document** in the collection that matches the given filter using the
    /// passed in client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to find a single matching document.
    /// - `update`: The update operations to apply (e.g., `$set`, `$unset`, etc.).
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) with `matched_count` and `modified_count`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let result = User::update_one_with_client(doc! { "age": 25 }, doc! { "$set": { "active": false } }, client).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_one_with_client(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<UpdateResult, OxiModError>;
    /// Deletes all documents in the collection that match the given filter using the passed in
    /// client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document specifying which documents to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of documents deleted.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let result = User::delete_with_client(doc! { "active": false }, client).await?;
    /// println!("Deleted {} users", result.deleted_count);
    /// ```
    async fn delete_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<DeleteResult, OxiModError>;
    /// Deletes the **first** document in the collection that matches the given filter using the
    /// passed in client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to find a single document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with details about the deletion.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let result = User::delete_one_with_client(doc! { "name": "user_a" }, client).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_one_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<DeleteResult, OxiModError>;
    /// Finds all documents in the collection that match the given filter using the passed in
    /// client.
    ///
    /// # Parameters
    /// - `filter`: A BSON query document used to match documents.
    ///
    /// # Returns
    /// - A `Vec<Self>` containing all matched documents.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let users = User::find_with_client(doc! { "active": true }, client).await?;
    /// assert!(!users.is_empty());
    /// ```
    async fn find_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<Vec<Self>, OxiModError>
    where
        Self: Sized;
    /// Finds the **first document** in the collection that matches the given filter using the
    /// passed in client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match a single document.
    ///
    /// # Returns
    /// - `Some(Self)` if a document is found, or `None` otherwise.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// if let Some(user) = User::find_one_with_client(doc! { "name": "user_a" }, client).await? {
    ///     println!("Found user: {}", user.name);
    /// }
    /// ```
    async fn find_one_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<Option<Self>, OxiModError>
    where
        Self: Sized;
    /// Finds a document in the collection by its MongoDB `_id` field using the passed in client.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document.
    ///
    /// # Returns
    /// - `Some(Self)` if found, or `None` if no document matches the ID.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let user = User::find_by_id_with_client(id, client).await?;
    /// if let Some(u) = user {
    ///     println!("Found: {}", u.name);
    /// }
    /// ```
    async fn find_by_id_with_client(
        id: ObjectId,
        client: &mongodb::Client,
    ) -> Result<Option<Self>, OxiModError>
    where
        Self: Sized;
    /// Updates a document by its MongoDB `_id` field using the passed in client.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document to update.
    /// - `update`: A BSON document containing update operations (e.g., `$set`).
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) with details on the matched and modified document.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::update_by_id_with_client(id, doc! { "$set": { "active": false } }, client).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_by_id_with_client(
        id: ObjectId,
        update: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<UpdateResult, OxiModError>;
    /// Deletes a document from the collection by its MongoDB `_id` field using the passed in
    /// client.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the deletion outcome.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::delete_by_id_with_client(id, client).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_by_id_with_client(
        id: ObjectId,
        client: &mongodb::Client,
    ) -> Result<DeleteResult, OxiModError>;
    /// Counts the number of documents in the collection that match the given filter using the
    /// passed in client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to match documents.
    ///
    /// # Returns
    /// - The number of matching documents as `u64`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let count = User::count_with_client(doc! { "active": true }, client).await?;
    /// println!("Active users: {}", count);
    /// ```
    async fn count_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<u64, OxiModError>;
    /// Checks if any document in the collection matches the given filter using the passed in
    /// client.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match against.
    ///
    /// # Returns
    /// - `true` if at least one document matches, `false` otherwise.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let exists = User::exists_with_client(doc! { "name": "user_a" }, client).await?;
    /// if exists {
    ///     println!("User exists!");
    /// }
    /// ```
    async fn exists_with_client(
        filter: impl Into<bson::Document> + Send,
        client: &mongodb::Client,
    ) -> Result<bool, OxiModError>;
    /// Deletes all documents from the model's collection using the passed in client.
    ///
    /// This is useful for resetting test data or clearing out a dataset.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of deleted documents.
    ///
    /// # Example
    /// ```rust,ignore
    /// let oxiclient = OxiClient::new(mongodb_uri.clone()).await?;
    /// let client = oxiclient.client().unwrap();
    /// let result = User::clear_with_client(client).await?;
    /// println!("Cleared {} documents", result.deleted_count);
    /// ```
    async fn clear_with_client(client: &mongodb::Client) -> Result<DeleteResult, OxiModError>;
    /// Retrieves the MongoDB collection associated with the model.
    ///
    /// This method is typically used internally by the framework, but it can be called
    /// directly when you need low-level access to the collection—such as for creating
    /// indexes manually or performing custom MongoDB operations not covered by the trait.
    ///
    /// # Returns
    /// - [`Collection<Document>`](https://docs.rs/mongodb/latest/mongodb/struct.Collection.html): A handle to the MongoDB collection.
    /// - [`OxiModError`](crate::error::oximod_error::OximodError): If the global client is not initialized or the collection name is missing.
    ///
    /// # Example
    /// ```rust,ignore
    /// let collection = User::get_collection()?;
    /// let count = collection.count_documents(doc! {}).await?;
    /// println!("Total documents: {}", count);
    /// ```
    fn get_collection() -> Result<Collection<Document>, OxiModError>;
    /// Inserts the current model instance into the MongoDB collection.
    ///
    /// # Returns
    /// - `ObjectId` of the inserted document.
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = user.save().await?;
    /// println!("Inserted user ID: {}", id);
    /// ```
    async fn save(&self) -> Result<ObjectId, OxiModError>;
    /// Updates all documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document specifying which documents to match.
    /// - `update`: A BSON document with the update operations to apply.
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) containing matched and modified counts.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = User::update(doc! { "active": false }, doc! { "$set": { "active": true } }).await?;
    /// assert_eq!(result.modified_count, 3);
    /// ```
    async fn update(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send,
    ) -> Result<UpdateResult, OxiModError>;
    /// Updates the **first document** in the collection that matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to find a single matching document.
    /// - `update`: The update operations to apply (e.g., `$set`, `$unset`, etc.).
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) with `matched_count` and `modified_count`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = User::update_one(doc! { "age": 25 }, doc! { "$set": { "active": false } }).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_one(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send,
    ) -> Result<UpdateResult, OxiModError>;
    /// Deletes all documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document specifying which documents to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of documents deleted.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = User::delete(doc! { "active": false }).await?;
    /// println!("Deleted {} users", result.deleted_count);
    /// ```
    async fn delete(filter: impl Into<bson::Document> + Send) -> Result<DeleteResult, OxiModError>;
    /// Deletes the **first** document in the collection that matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to find a single document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with details about the deletion.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = User::delete_one(doc! { "name": "user_a" }).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_one(
        filter: impl Into<bson::Document> + Send,
    ) -> Result<DeleteResult, OxiModError>;
    /// Finds all documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON query document used to match documents.
    ///
    /// # Returns
    /// - A `Vec<Self>` containing all matched documents.
    ///
    /// # Example
    /// ```rust,ignore
    /// let users = User::find(doc! { "active": true }).await?;
    /// assert!(!users.is_empty());
    /// ```
    async fn find(filter: impl Into<bson::Document> + Send) -> Result<Vec<Self>, OxiModError>
    where
        Self: Sized;
    /// Finds the **first document** in the collection that matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match a single document.
    ///
    /// # Returns
    /// - `Some(Self)` if a document is found, or `None` otherwise.
    ///
    /// # Example
    /// ```rust,ignore
    /// if let Some(user) = User::find_one(doc! { "name": "user_a" }).await? {
    ///     println!("Found user: {}", user.name);
    /// }
    /// ```
    async fn find_one(
        filter: impl Into<bson::Document> + Send,
    ) -> Result<Option<Self>, OxiModError>
    where
        Self: Sized;
    /// Finds a document in the collection by its MongoDB `_id` field.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document.
    ///
    /// # Returns
    /// - `Some(Self)` if found, or `None` if no document matches the ID.
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let user = User::find_by_id(id).await?;
    /// if let Some(u) = user {
    ///     println!("Found: {}", u.name);
    /// }
    /// ```
    async fn find_by_id(id: ObjectId) -> Result<Option<Self>, OxiModError>
    where
        Self: Sized;
    /// Updates a document by its MongoDB `_id` field.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document to update.
    /// - `update`: A BSON document containing update operations (e.g., `$set`).
    ///
    /// # Returns
    /// - [`UpdateResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.UpdateResult.html) with details on the matched and modified document.
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::update_by_id(id, doc! { "$set": { "active": false } }).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_by_id(
        id: ObjectId,
        update: impl Into<bson::Document> + Send,
    ) -> Result<UpdateResult, OxiModError>;
    /// Deletes a document from the collection by its MongoDB `_id` field.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the deletion outcome.
    ///
    /// # Example
    /// ```rust,ignore
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::delete_by_id(id).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_by_id(id: ObjectId) -> Result<DeleteResult, OxiModError>;
    /// Counts the number of documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to match documents.
    ///
    /// # Returns
    /// - The number of matching documents as `u64`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let count = User::count(doc! { "active": true }).await?;
    /// println!("Active users: {}", count);
    /// ```
    async fn count(filter: impl Into<bson::Document> + Send) -> Result<u64, OxiModError>;
    /// Checks if any document in the collection matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match against.
    ///
    /// # Returns
    /// - `true` if at least one document matches, `false` otherwise.
    ///
    /// # Example
    /// ```rust,ignore
    /// let exists = User::exists(doc! { "name": "user_a" }).await?;
    /// if exists {
    ///     println!("User exists!");
    /// }
    /// ```
    async fn exists(filter: impl Into<bson::Document> + Send) -> Result<bool, OxiModError>;
    /// Deletes all documents from the model's collection.
    ///
    /// This is useful for resetting test data or clearing out a dataset.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of deleted documents.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = User::clear().await?;
    /// println!("Cleared {} documents", result.deleted_count);
    /// ```
    async fn clear() -> Result<DeleteResult, OxiModError>;
}
