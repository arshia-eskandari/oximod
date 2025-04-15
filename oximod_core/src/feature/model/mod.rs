use async_trait;
use mongodb::{bson::{self, oid::ObjectId}, results::{DeleteResult, UpdateResult}};
use crate::error::conn_error::MongoDbError;

#[async_trait::async_trait]
pub trait Model {
    /// Inserts the current model instance into the MongoDB collection.
    ///
    /// # Returns
    /// - `ObjectId` of the inserted document.
    ///
    /// # Example
    /// ```rust, no_run
    /// let id = user.save().await?;
    /// println!("Inserted user ID: {}", id);
    /// ```
    async fn save(&self) -> Result<ObjectId, MongoDbError>;
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
    /// ```rust, no_run
    /// let result = User::update(doc! { "active": false }, doc! { "$set": { "active": true } }).await?;
    /// assert_eq!(result.modified_count, 3);
    /// ```
    async fn update(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send
    ) -> Result<UpdateResult, MongoDbError>;
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
    /// ```rust, no_run
    /// let result = User::update_one(doc! { "age": 25 }, doc! { "$set": { "active": false } }).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_one(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send
    ) -> Result<UpdateResult, MongoDbError>;
    /// Deletes all documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document specifying which documents to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of documents deleted.
    ///
    /// # Example
    /// ```rust, no_run
    /// let result = User::delete(doc! { "active": false }).await?;
    /// println!("Deleted {} users", result.deleted_count);
    /// ```
    async fn delete(filter: impl Into<bson::Document> + Send) -> Result<DeleteResult, MongoDbError>;
    /// Deletes the **first** document in the collection that matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to find a single document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with details about the deletion.
    ///
    /// # Example
    /// ```rust, no_run
    /// let result = User::delete_one(doc! { "name": "user_a" }).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_one(filter: impl Into<bson::Document> + Send) -> Result<DeleteResult, MongoDbError>;
    /// Finds all documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON query document used to match documents.
    ///
    /// # Returns
    /// - A `Vec<Self>` containing all matched documents.
    ///
    /// # Example
    /// ```rust, no_run
    /// let users = User::find(doc! { "active": true }).await?;
    /// assert!(!users.is_empty());
    /// ```
    async fn find(filter: impl Into<bson::Document> + Send) -> Result<Vec<Self>, MongoDbError>
        where Self: Sized;
    /// Finds the **first document** in the collection that matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match a single document.
    ///
    /// # Returns
    /// - `Some(Self)` if a document is found, or `None` otherwise.
    ///
    /// # Example
    /// ```rust, no_run
    /// if let Some(user) = User::find_one(doc! { "name": "user_a" }).await? {
    ///     println!("Found user: {}", user.name);
    /// }
    /// ``` 
    async fn find_one(
        filter: impl Into<bson::Document> + Send
    ) -> Result<Option<Self>, MongoDbError>
        where Self: Sized;
    /// Finds a document in the collection by its MongoDB `_id` field.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document.
    ///
    /// # Returns
    /// - `Some(Self)` if found, or `None` if no document matches the ID.
    ///
    /// # Example
    /// ```rust, no_run
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let user = User::find_by_id(id).await?;
    /// if let Some(u) = user {
    ///     println!("Found: {}", u.name);
    /// }
    /// ``` 
    async fn find_by_id(id: ObjectId) -> Result<Option<Self>, MongoDbError> where Self: Sized;
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
    /// ```rust, no_run
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::update_by_id(id, doc! { "$set": { "active": false } }).await?;
    /// assert_eq!(result.matched_count, 1);
    /// ```
    async fn update_by_id(
        id: ObjectId,
        update: impl Into<bson::Document> + Send
    ) -> Result<UpdateResult, MongoDbError>;
    /// Deletes a document from the collection by its MongoDB `_id` field.
    ///
    /// # Parameters
    /// - `id`: The [`ObjectId`](https://docs.rs/mongodb/latest/mongodb/bson/oid/struct.ObjectId.html) of the document to delete.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the deletion outcome.
    ///
    /// # Example
    /// ```rust, no_run
    /// let id = ObjectId::parse_str("652efcddfc13ae2c82000001")?;
    /// let result = User::delete_by_id(id).await?;
    /// assert_eq!(result.deleted_count, 1);
    /// ```
    async fn delete_by_id(id: ObjectId) -> Result<DeleteResult, MongoDbError>;
    /// Counts the number of documents in the collection that match the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document used to match documents.
    ///
    /// # Returns
    /// - The number of matching documents as `u64`.
    ///
    /// # Example
    /// ```rust, no_run
    /// let count = User::count(doc! { "active": true }).await?;
    /// println!("Active users: {}", count);
    /// ```
    async fn count(filter: impl Into<bson::Document> + Send) -> Result<u64, MongoDbError>;
    /// Checks if any document in the collection matches the given filter.
    ///
    /// # Parameters
    /// - `filter`: A BSON document to match against.
    ///
    /// # Returns
    /// - `true` if at least one document matches, `false` otherwise.
    ///
    /// # Example
    /// ```rust, no_run
    /// let exists = User::exists(doc! { "name": "user_a" }).await?;
    /// if exists {
    ///     println!("User exists!");
    /// }
    /// ```
    async fn exists(filter: impl Into<bson::Document> + Send) -> Result<bool, MongoDbError>;
    /// Deletes all documents from the model's collection.
    ///
    /// This is useful for resetting test data or clearing out a dataset.
    ///
    /// # Returns
    /// - [`DeleteResult`](https://docs.rs/mongodb/latest/mongodb/results/struct.DeleteResult.html) with the number of deleted documents.
    ///
    /// # Example
    /// ```rust, no_run
    /// let result = User::clear().await?;
    /// println!("Cleared {} documents", result.deleted_count);
    /// ```
    async fn clear() -> Result<DeleteResult, MongoDbError>;     
}
