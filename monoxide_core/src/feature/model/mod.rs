use async_trait;
use mongodb::bson::{self, oid::ObjectId};
use crate::error::conn_error::MongoDbError;

#[async_trait::async_trait]
pub trait Model {
    async fn save(&self) -> Result<(), MongoDbError>;
    async fn update(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send
    ) -> Result<(), MongoDbError>;
    async fn update_one(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send
    ) -> Result<(), MongoDbError>;
    async fn delete(filter: impl Into<bson::Document> + Send) -> Result<u64, MongoDbError>;
    async fn delete_one(filter: impl Into<bson::Document> + Send) -> Result<bool, MongoDbError>;
    async fn find(filter: impl Into<bson::Document> + Send) -> Result<Vec<Self>, MongoDbError>
        where Self: Sized;
    async fn find_one(
        filter: impl Into<bson::Document> + Send
    ) -> Result<Option<Self>, MongoDbError>
        where Self: Sized;
    async fn find_by_id(id: ObjectId) -> Result<Option<Self>, MongoDbError> where Self: Sized;
    async fn update_by_id(
        id: ObjectId,
        update: impl Into<bson::Document> + Send
    ) -> Result<(), MongoDbError>;
    async fn delete_by_id(id: ObjectId) -> Result<bool, MongoDbError>;
    async fn count(filter: impl Into<bson::Document> + Send) -> Result<u64, MongoDbError>;
    async fn exists(filter: impl Into<bson::Document> + Send) -> Result<bool, MongoDbError>;
}
