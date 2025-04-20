use thiserror::Error;

/// Represents all possible errors returned by OxiMod during database operations.
#[derive(Debug, Error)]
pub enum OximodError {
    /// Failed to connect to the MongoDB server.
    /// This may indicate an invalid URI, network issues, or server downtime.
    #[error("Failed to connect to db: {0}")]
    ConnectionError(String),

    /// Failed to initialize the global MongoDB client.
    /// This typically happens when trying to set it more than once.
    #[error("Failed to set CLIENT")]
    GlobalClientInitError(String),

    /// Attempted to retrieve the global MongoDB client before initialization.
    /// Make sure to call `set_global_client()` before performing any database operations.
    #[error("CLIENT not found: {0}")]
    GlobalClientMissing(String),

    /// Error serializing or deserializing between MongoDB documents and Rust structs.
    /// This usually indicates a mismatch between struct fields and BSON types.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// An error occurred while executing an aggregation pipeline.
    /// This may result from malformed pipeline stages or collection access issues.
    #[error("Aggregation error: {0}")]
    AggregationError(String),
}
