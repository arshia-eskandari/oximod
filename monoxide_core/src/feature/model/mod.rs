use async_trait;
use crate::error::client_error::MongoClientError;

#[async_trait::async_trait]
pub trait Model {
    async fn save(&self) -> Result<(), MongoClientError>;
}
