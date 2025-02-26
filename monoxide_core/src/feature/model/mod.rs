use async_trait;
use crate::error::conn_error::MongoDbError;

#[async_trait::async_trait]
pub trait Model {
    async fn save(&self) -> Result<(), MongoDbError>;
}
