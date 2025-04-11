use async_trait;
use mongodb::bson;
use crate::error::conn_error::MongoDbError;

#[async_trait::async_trait]
pub trait Model {
    async fn save(&self) -> Result<(), MongoDbError>;
    async fn update(
        filter: impl Into<bson::Document> + Send,
        update: impl Into<bson::Document> + Send
    ) -> Result<(), MongoDbError>;
}
