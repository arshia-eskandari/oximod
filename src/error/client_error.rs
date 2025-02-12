use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoClientError {
    #[error("Failed to connect to db: {0}")] ConnectionError(String),

    #[error("Invalid input provided: {0}")] Validation(String),

    #[error("Database query failed: {0}")] QueryError(String),

    #[error("An unexpected error occurred: {0}")] Unexpected(String),

    #[error("Failed to set CLIENT")] SetClientError(String),

    #[error("Database client not found: {0}")] ClientNotFound(String),
}
