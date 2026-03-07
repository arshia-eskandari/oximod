use crate::error::oximod_error::OxiModError;
use async_trait;
use mongodb::{
    Collection,
    bson::{Document, oid::ObjectId},
    results::DeleteResult,
};

// TODO: modify documentation and add accurate examples
/// An asynchronous trait for MongoDB models enabling CRUD operations, typically implemented via the #[derive(Model)] macro.
#[async_trait::async_trait]
pub trait Model
where
    Self: Send + Sync + Sized,
{
    fn get_collection_from(client: &mongodb::Client) -> Result<Collection<Self>, OxiModError>;
    fn get_document_collection_from(
        client: &mongodb::Client,
    ) -> Result<Collection<Document>, OxiModError> {
        Ok(Self::get_collection_from(client)?.clone_with_type::<Document>())
    }
    async fn save_from(&self, client: &mongodb::Client) -> Result<ObjectId, OxiModError>;
    async fn clear_from(client: &mongodb::Client) -> Result<DeleteResult, OxiModError>;
    fn get_collection() -> Result<Collection<Self>, OxiModError>;
    fn get_document_collection() -> Result<Collection<Document>, OxiModError> {
        Ok(Self::get_collection()?.clone_with_type::<Document>())
    }
    async fn save(&self) -> Result<ObjectId, OxiModError>;
    async fn clear() -> Result<DeleteResult, OxiModError>;
}
