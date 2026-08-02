use super::query_error::QueryError;
use std::error::Error as StdError;
use thiserror::Error;

/// A boxed error type used by OxiMod to preserve underlying sources.
///
/// This keeps OxiMod errors compatible with:
///
/// - multithreaded asynchronous runtimes through `Send + Sync`,
/// - downstream error reporting through [`std::error::Error::source`].
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Represents a validation failure for a specific model field.
///
/// Each error records the model field that failed validation and a
/// human-readable description of the violated rule.
///
/// Validation errors are returned through [`OxiModError::Validation`].
#[derive(Debug, Default)]
pub struct ValidationError {
    /// The model field that failed validation.
    pub field: String,

    /// A human-readable description of the validation failure.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl ValidationError {
    /// Creates a validation error for `field` with the supplied message.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Contains all field-level validation errors collected for a model.
#[derive(Debug)]
pub struct ValidationErrors(
    /// The collected field-level validation errors.
    pub Vec<ValidationError>,
);

impl ValidationErrors {
    /// Creates a collection of validation errors.
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

/// Represents errors returned by OxiMod operations.
///
/// Driver and runtime failures preserve their underlying source errors, while
/// validation and query-configuration failures remain directly inspectable
/// through their dedicated variants and accessor methods.
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
    /// This variant has no source error because validation failures describe
    /// model data rather than a driver or runtime failure.
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
    /// Creates a connection error with a message and underlying source error.
    pub fn connection(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Connection {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates a global-client initialization error with a message.
    pub fn global_client_init(msg: impl Into<String>) -> Self {
        Self::GlobalClientInit { msg: msg.into() }
    }

    /// Creates an error indicating that the global client is unavailable.
    pub fn global_client_missing(msg: impl Into<String>) -> Self {
        Self::GlobalClientMissing { msg: msg.into() }
    }

    /// Creates a serialization error with a message and underlying source error.
    pub fn serialization(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Serialization {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates an aggregation error with a message and underlying source error.
    pub fn aggregation(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Aggregation {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates an index error with a message and underlying source error.
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

    /// Creates a database error with a message and underlying source error.
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
    /// Returns `Some` for [`OxiModError::Validation`] and `None` for every
    /// other variant.
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
