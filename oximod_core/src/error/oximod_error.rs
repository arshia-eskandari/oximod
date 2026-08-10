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
/// # Failure-class contract
///
/// Variants identify the **class of failure**: MongoDB driver errors
/// produced while executing OxiMod database operations are classified by
/// failure class rather than by the method that was executing, with a
/// deterministic precedence:
///
/// 1. [`Connection`](OxiModError::Connection) — connectivity /
///    client-infrastructure failure, during any operation;
/// 2. [`Serialization`](OxiModError::Serialization) — BSON encoding or
///    decoding failure, in either direction;
/// 3. [`Index`](OxiModError::Index) / [`Aggregation`](OxiModError::Aggregation)
///    — remaining failures of the failing operation's domain;
/// 4. [`Database`](OxiModError::Database) — the conservative fallback for
///    every remaining driver failure.
///
/// MongoDB client construction and setup remain a connection concern
/// reported directly as [`Connection`](OxiModError::Connection), outside the
/// operation-time classification above. The `GlobalClientInit`,
/// `GlobalClientMissing`, `Validation`, `Custom`, and `Query` variants are
/// not selected by the operation-time driver classifier.
///
/// Driver and runtime failures preserve their underlying source errors,
/// while validation and query-configuration failures remain directly
/// inspectable through their dedicated variants and accessor methods. The
/// original [`mongodb::error::Error`] remains available through
/// [`std::error::Error::source`] and can be downcast to recover server
/// detail such as duplicate-key code 11000:
///
/// ```rust,ignore
/// use std::error::Error as _;
///
/// let driver_error = error
///     .source()
///     .and_then(|source| source.downcast_ref::<mongodb::error::Error>());
/// ```
///
/// `Display` output carries human-readable operation context only; it is
/// not a machine-classification API. Classify by matching the variant and,
/// when driver detail is needed, by inspecting `source()`.
///
/// Variant matchers written for OxiMod 0.3.0 may observe different arms:
/// 0.3.0 selected variants by call site, and several mappings changed when
/// the failure-class contract was adopted.
#[derive(Debug, Error)]
pub enum OxiModError {
    /// MongoDB client/connectivity infrastructure failure.
    ///
    /// Covers direct client construction and setup failures — such as an
    /// invalid connection URI or TLS configuration while creating an
    /// `OxiClient` — as well as operation-time connectivity failures:
    /// connection establishment, authentication, DNS resolution, TLS
    /// configuration, server selection, transport I/O, and connection-pool
    /// failure, during any operation, including index establishment.
    ///
    /// This variant classifies the failure; it does not guarantee that the
    /// logical operation was never sent to or never reached MongoDB, and it
    /// does not make the operation safe to retry. Retry safety depends on
    /// the specific operation, its idempotency, and application policy.
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

    /// Rust/BSON encoding or decoding failure, independent of call site.
    ///
    /// Covers both directions: encoding a model for a write (for example a
    /// value BSON cannot represent) and decoding a stored document into the
    /// model type (mismatched BSON types, schema drift, invalid data for the
    /// expected Rust type). The same undeserializable document classifies
    /// identically through every read path, including `find_by_id`,
    /// `query().first()`, and `query().all()`.
    #[error("Serialization error: {msg}")]
    Serialization {
        /// Human-readable context describing the serialization step that failed.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// Non-connectivity, non-serialization aggregation-domain failure.
    ///
    /// Applies only after the [`Connection`](OxiModError::Connection) and
    /// [`Serialization`](OxiModError::Serialization) precedence: a
    /// connectivity failure during an aggregation classifies as
    /// `Connection`, while a server-side pipeline rejection or execution
    /// failure classifies here.
    #[error("Aggregation error: {msg}")]
    Aggregation {
        /// Human-readable context describing the aggregation step that failed.
        msg: String,
        /// The underlying error.
        #[source]
        source: BoxError,
    },

    /// Non-connectivity, non-serialization index-domain failure.
    ///
    /// Covers server-side rejection of an index operation, such as invalid
    /// or conflicting index specifications and insufficient permissions.
    /// Applies only after the [`Connection`](OxiModError::Connection) and
    /// [`Serialization`](OxiModError::Serialization) precedence: a
    /// connectivity failure during index establishment classifies as
    /// `Connection`.
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

    /// General MongoDB/driver operation failure that is not classified as
    /// `Connection`, `Serialization`, `Index`, or `Aggregation`.
    ///
    /// Covers server-side operation failures such as write rejections —
    /// including duplicate key, whose server code 11000 remains recoverable
    /// through [`std::error::Error::source`] — command failures, and
    /// write-concern failures.
    ///
    /// This variant is also the conservative fallback for driver failures
    /// OxiMod does not specifically recognize; it does not guarantee that
    /// the server definitely received or rejected the operation.
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
    ///
    /// This helper sets the variant unconditionally; it performs no
    /// classification. OxiMod's own operations classify driver failures by
    /// failure class before choosing a variant.
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
    ///
    /// This helper sets the variant unconditionally; it performs no
    /// classification.
    pub fn serialization(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Serialization {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates an aggregation error with a message and underlying source error.
    ///
    /// This helper sets the variant unconditionally; it performs no
    /// classification.
    pub fn aggregation(msg: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::Aggregation {
            msg: msg.into(),
            source: source.into(),
        }
    }

    /// Creates an index error with a message and underlying source error.
    ///
    /// This helper sets the variant unconditionally; it performs no
    /// classification.
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
    ///
    /// This helper sets the variant unconditionally; it performs no
    /// classification. OxiMod's own operations classify driver failures by
    /// failure class before choosing a variant.
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
