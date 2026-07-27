use super::query_error::QueryError;
use std::error::Error as StdError;
use thiserror::Error;

/// A boxed error type used by OxiMod to preserve underlying sources.
///
/// This keeps OxiMod errors compatible with:
/// - multi-threaded async runtimes (Send + Sync)
/// - downstream applications that need error chaining via `source()`
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Represents a validation failure for a specific model field.
///
/// Each `ValidationError` contains:
/// - the name of the field that failed validation
/// - a human-readable message describing the violation
///
/// This type is typically returned as part of
/// `OxiModError::Validation(Vec<ValidationError>)`.
#[derive(Debug, Default)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Represents all validation errors for all fields on a model
#[derive(Debug)]
pub struct ValidationErrors(pub Vec<ValidationError>);

impl ValidationErrors {
    pub fn new(validation_errors: impl Into<Vec<ValidationError>>) -> Self {
        Self(validation_errors.into())
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, err) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{err}")?;
        }
        Ok(())
    }
}

/// Represents all possible errors returned by OxiMod during database operations.
///
/// # Design goals
/// - **Stable Rust compatibility** (no nightly features)
/// - **Error chaining** via `#[source]` where applicable
/// - **Human-friendly context** via `msg`
/// - **Ergonomic construction** via helper constructors
///
/// # Downstream usage
/// Applications can inspect causes via `.source()` and display a full chain using
/// their preferred error/reporting stack (e.g. `anyhow`, `eyre`, `tracing`).
#[derive(Debug, Error)]
pub enum OxiModError {
    /// Failed to connect to the MongoDB server.
    ///
    /// Common causes:
    /// - invalid connection URI
    /// - network connectivity issues
    /// - authentication failure
    /// - server unavailable
    #[error("Failed to connect to db: {msg}")]
    Connection {
        /// Human-readable context describing *what* was being attempted.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// Failed to initialize the global MongoDB client.
    ///
    /// Typically occurs when attempting to set the global client more than once
    /// or when the underlying synchronization primitive fails.
    #[error("Failed to set CLIENT: {msg}")]
    GlobalClientInit {
        /// Human-readable context describing the initialization failure.
        msg: String,
    },

    /// Attempted to retrieve the global MongoDB client before it was initialized.
    ///
    /// Ensure your application calls the global initialization routine before
    /// performing any database operations that depend on it.
    #[error("CLIENT not found: {msg}")]
    GlobalClientMissing {
        /// Human-readable context explaining what was requested.
        msg: String,
    },

    /// Error serializing or deserializing between MongoDB documents and Rust structs.
    ///
    /// Common causes:
    /// - mismatched BSON types
    /// - schema drift
    /// - invalid data for the expected Rust type
    #[error("Serialization error: {msg}")]
    Serialization {
        /// Human-readable context describing the serialization step that failed.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// An error occurred while executing an aggregation pipeline.
    ///
    /// Common causes:
    /// - malformed pipeline stages
    /// - collection access issues
    /// - server-side execution errors
    #[error("Aggregation error: {msg}")]
    Aggregation {
        /// Human-readable context describing the aggregation step that failed.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// An error occurred during index creation, deletion, or retrieval.
    ///
    /// Common causes:
    /// - invalid index specifications
    /// - duplicate definitions
    /// - insufficient permissions
    /// - server-side errors
    #[error("Index error: {msg}")]
    Index {
        /// Human-readable context describing the index operation that failed.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// A validation rule was violated.
    ///
    /// Examples:
    /// - `required` field missing
    /// - `min_length` / `max_length` violated
    /// - bounds like `min` / `max` violated
    /// - `pattern` mismatch
    ///
    /// This variant intentionally has no `source` because validation failures
    /// are domain errors, not driver/system errors.
    #[error("Validation errors: {0}")]
    Validation(ValidationErrors),

    /// Error returned when a database operation fails.
    ///
    /// This variant is used when an operation involving MongoDB fails,
    /// such as insert, update, delete, find, aggregation, or other
    /// driver-level calls.
    ///
    /// The underlying error produced by the MongoDB driver or runtime
    /// is stored as the source.
    #[error("Database operation failed: {msg}")]
    Database {
        /// A human-readable description of the database error.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// Error returned from user-defined logic.
    ///
    /// This variant is intended for errors originating from user code,
    /// such as hooks, custom validators, or other application-specific
    /// logic executed during model operations.
    ///
    /// Unlike other variants, which represent errors produced internally
    /// by OxiMod, this variant allows users to return domain-specific
    /// failures while still using the `OxiModError` type.
    ///
    /// A source error may optionally be attached.
    #[error("Custom error: {msg}")]
    Custom {
        /// A human-readable description of the error.
        msg: String,
        /// The underlying error.
        #[source]
        source: Option<BoxError>,
    },

    /// The typed query contains invalid configuration.
    ///
    /// Examples include:
    /// - page number zero
    /// - page size zero
    /// - pagination offset overflow
    /// - a limit outside MongoDB's supported range
    #[error(transparent)]
    Query(#[from] QueryError),
}

impl OxiModError {
    /// Create a `Connection` error with a message and an underlying source error.
    pub fn connection(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Connection {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Create a `GlobalClientInit` error with a message and an underlying source error.
    pub fn global_client_init(msg: impl Into<String>) -> Self {
        Self::GlobalClientInit { msg: msg.into() }
    }

    /// Create a `GlobalClientMissing` error with a message.
    pub fn global_client_missing(msg: impl Into<String>) -> Self {
        Self::GlobalClientMissing { msg: msg.into() }
    }

    /// Create a `Serialization` error with a message and an underlying source error.
    pub fn serialization(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Serialization {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Create an `Aggregation` error with a message and an underlying source error.
    pub fn aggregation(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Aggregation {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Create an `Index` error with a message and an underlying source error.
    pub fn index(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Index {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates a validation error for a single field.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation(ValidationErrors(vec![ValidationError::new(field, message)]))
    }

    /// Creates a validation error containing multiple field failures.
    pub fn validations(errors: Vec<ValidationError>) -> Self {
        Self::Validation(ValidationErrors(errors))
    }

    /// Create a `Database` error with a message and an underlying source error.
    pub fn database(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Database {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates a custom error without a source.
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom {
            msg: msg.into(),
            source: None,
        }
    }

    /// Creates a custom error with an underlying source error.
    pub fn custom_with_source(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Custom {
            msg: msg.into(),
            source: Some(source.into()),
        }
    }

    /// Returns all validation errors if this is a `Validation` error.
    ///
    /// This provides convenient access to the underlying field-level
    /// validation failures without requiring pattern matching.
    ///
    /// Returns `Some(&[ValidationError])` if the error is of type
    /// `OxiModError::Validation`, otherwise `None`.
    pub fn validation_errors(&self) -> Option<&[ValidationError]> {
        match self {
            Self::Validation(errors) => Some(&errors.0),
            _ => None,
        }
    }

    /// Returns the underlying query error when this is an
    /// `OxiModError::Query` variant.
    pub fn query_error(&self) -> Option<&QueryError> {
        match self {
            Self::Query(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OxiModError;
    use crate::error::query_error::QueryError;

    #[test]
    fn query_error_converts_into_oximod_error() {
        let error: OxiModError = QueryError::InvalidPageNumber { page: 0 }.into();

        assert_eq!(error.to_string(), "page number must be at least 1, got 0");

        assert!(matches!(
            error.query_error(),
            Some(QueryError::InvalidPageNumber { page: 0 })
        ));
    }
}
