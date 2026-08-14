//! Centralized failure-class classification for MongoDB driver errors.
//!
//! This module is OxiMod-internal macro-support infrastructure. It is the
//! single owner of the policy that maps a [`mongodb::error::Error`] produced
//! during an operation onto the public [`OxiModError`] failure-class
//! contract; generated and non-generated call sites both route through it
//! instead of matching [`mongodb::error::ErrorKind`] locally.
//!
//! It is not part of OxiMod's supported public API. Consumers classify
//! failures through the public [`OxiModError`] variants and, for driver
//! detail, through [`std::error::Error::source`].
//!
//! The deterministic precedence is:
//!
//! 1. `Connection` — connectivity/client-infrastructure driver kinds;
//! 2. `Serialization` — BSON encode/decode driver kinds;
//! 3. the operation domain (`Index`, `Aggregation`, or `BulkWrite`) for
//!    every remaining kind, when the failing operation carries that domain;
//! 4. `Database` — the conservative fallback for every remaining kind,
//!    including future kinds of the `#[non_exhaustive]` driver enum.
//!
//! The original driver error is always preserved unmodified as the chosen
//! variant's `source`.

use mongodb::error::ErrorKind;

use super::oximod_error::OxiModError;

/// Operation domain of a failing driver call.
///
/// The domain refines otherwise-general operational failures into the
/// domain-specific `Index` and `Aggregation` variants. It never overrides the
/// `Connection` or `Serialization` classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationDomain {
    /// An ordinary CRUD, query, or count operation.
    General,

    /// An index-domain operation such as index establishment.
    Index,

    /// An aggregation-domain operation.
    ///
    /// Carried by the aggregation execution path in
    /// [`crate::aggregation::Aggregation`], so non-connectivity,
    /// non-serialization driver failures during aggregation classify as
    /// [`OxiModError::Aggregation`].
    Aggregation,

    /// A bulk-write-domain operation.
    ///
    /// Carried by the bulk-write materialization and execution paths in
    /// [`crate::bulk_write::BulkWrite`], so non-connectivity,
    /// non-serialization driver failures during a bulk write classify as
    /// [`OxiModError::BulkWrite`].
    BulkWrite,
}

/// Failure class a driver error kind belongs to, before domain refinement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureClass {
    Connection,
    Serialization,
    Operational,
}

/// Maps a driver error kind onto its failure class.
///
/// The wildcard arm is the conservative fallback required by the
/// `#[non_exhaustive]` driver enum: unrecognized current or future kinds
/// classify as operational failures, never as `Connection` or
/// `Serialization`.
fn failure_class(kind: &ErrorKind) -> FailureClass {
    match kind {
        ErrorKind::Io(_)
        | ErrorKind::ServerSelection { .. }
        | ErrorKind::ConnectionPoolCleared { .. }
        | ErrorKind::DnsResolve { .. }
        | ErrorKind::InvalidTlsConfig { .. }
        | ErrorKind::Authentication { .. } => FailureClass::Connection,
        ErrorKind::BsonSerialization(_) | ErrorKind::BsonDeserialization(_) => {
            FailureClass::Serialization
        }
        _ => FailureClass::Operational,
    }
}

/// Classifies an operation-time MongoDB driver error into the failure-class
/// contract.
///
/// `msg` carries human-readable operation context only; it never influences
/// classification. The original `error` is preserved unmodified as the
/// returned variant's source, so consumers can keep downcasting to
/// [`mongodb::error::Error`] and recover server detail such as duplicate-key
/// code 11000.
///
/// This function is macro-support infrastructure, not supported public API.
#[doc(hidden)]
pub fn classify_driver_error(
    msg: impl Into<String>,
    domain: OperationDomain,
    error: mongodb::error::Error,
) -> OxiModError {
    match failure_class(&error.kind) {
        FailureClass::Connection => OxiModError::connection(msg, error),
        FailureClass::Serialization => OxiModError::serialization(msg, error),
        FailureClass::Operational => match domain {
            OperationDomain::Index => OxiModError::index(msg, error),
            OperationDomain::Aggregation => OxiModError::aggregation(msg, error),
            OperationDomain::BulkWrite => OxiModError::bulk_write(msg, error),
            OperationDomain::General => OxiModError::database(msg, error),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::sync::Arc;

    use mongodb::bson::{Bson, bson, doc};
    use mongodb::error::{CommandError, Error as DriverError, ErrorKind, WriteFailure};

    use super::{OperationDomain, classify_driver_error};
    use crate::error::oximod_error::OxiModError;

    /// Wraps a soundly constructible driver error kind into a driver error.
    fn driver_error(kind: ErrorKind) -> DriverError {
        DriverError::from(kind)
    }

    fn io_kind() -> ErrorKind {
        ErrorKind::Io(Arc::new(std::io::Error::other("connection refused")))
    }

    fn bson_serialization_kind() -> ErrorKind {
        let error = mongodb::bson::to_bson(&u64::MAX)
            .expect_err("u64::MAX should not be encodable as BSON");
        ErrorKind::BsonSerialization(error)
    }

    fn bson_deserialization_kind() -> ErrorKind {
        let error = mongodb::bson::from_bson::<String>(Bson::Int32(1))
            .expect_err("an Int32 should not deserialize as a String");
        ErrorKind::BsonDeserialization(error)
    }

    /// Constructs a server command rejection through its public
    /// `Deserialize` implementation.
    fn command_kind(code: i32) -> ErrorKind {
        let command_error: CommandError = mongodb::bson::from_document(doc! {
            "code": code,
            "codeName": "TestCommandFailure",
            "errmsg": "server rejected the command",
        })
        .expect("a CommandError should deserialize from a server-shaped document");
        ErrorKind::Command(command_error)
    }

    /// Constructs a duplicate-key write failure through its public
    /// `Deserialize` implementation.
    fn duplicate_key_kind() -> ErrorKind {
        let failure: WriteFailure = mongodb::bson::from_bson(bson!({
            "WriteError": {
                "code": 11000,
                "errmsg": "E11000 duplicate key error",
            }
        }))
        .expect("a WriteFailure should deserialize from a server-shaped value");
        ErrorKind::Write(failure)
    }

    // Connection family: connectivity/client-infrastructure kinds classify
    // as Connection in every operation domain. The kinds that cannot be
    // soundly constructed here (ServerSelection, DnsResolve,
    // InvalidTlsConfig, Authentication, ConnectionPoolCleared) are proven
    // behaviorally by the integration matrix in
    // `oximod/tests/error_classification.rs`.
    #[test]
    fn io_failure_classifies_as_connection() {
        let error = classify_driver_error("ctx", OperationDomain::General, driver_error(io_kind()));

        assert!(matches!(error, OxiModError::Connection { .. }));
    }

    #[test]
    fn connection_precedence_outranks_index_domain() {
        let error = classify_driver_error("ctx", OperationDomain::Index, driver_error(io_kind()));

        assert!(matches!(error, OxiModError::Connection { .. }));
    }

    #[test]
    fn connection_precedence_outranks_aggregation_domain() {
        let error =
            classify_driver_error("ctx", OperationDomain::Aggregation, driver_error(io_kind()));

        assert!(matches!(error, OxiModError::Connection { .. }));
    }

    #[test]
    fn connection_precedence_outranks_bulk_write_domain() {
        let error =
            classify_driver_error("ctx", OperationDomain::BulkWrite, driver_error(io_kind()));

        assert!(matches!(error, OxiModError::Connection { .. }));
    }

    // Serialization family: both encode and decode kinds classify as
    // Serialization, and outrank the operation domain.
    #[test]
    fn bson_encode_failure_classifies_as_serialization() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::General,
            driver_error(bson_serialization_kind()),
        );

        assert!(matches!(error, OxiModError::Serialization { .. }));
    }

    #[test]
    fn bson_decode_failure_classifies_as_serialization() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::General,
            driver_error(bson_deserialization_kind()),
        );

        assert!(matches!(error, OxiModError::Serialization { .. }));
    }

    #[test]
    fn serialization_precedence_outranks_index_domain() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::Index,
            driver_error(bson_deserialization_kind()),
        );

        assert!(matches!(error, OxiModError::Serialization { .. }));
    }

    #[test]
    fn serialization_precedence_outranks_bulk_write_domain() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::BulkWrite,
            driver_error(bson_serialization_kind()),
        );

        assert!(matches!(error, OxiModError::Serialization { .. }));
    }

    // Operational family: server rejections and write failures classify as
    // Database in the general domain.
    #[test]
    fn command_failure_classifies_as_database() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::General,
            driver_error(command_kind(2)),
        );

        assert!(matches!(error, OxiModError::Database { .. }));
    }

    #[test]
    fn duplicate_key_write_failure_classifies_as_database() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::General,
            driver_error(duplicate_key_kind()),
        );

        assert!(matches!(error, OxiModError::Database { .. }));
    }

    // Domain refinement: operational failures refine to the operation
    // domain's variant.
    #[test]
    fn index_domain_refines_operational_failure_to_index() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::Index,
            driver_error(command_kind(85)),
        );

        assert!(matches!(error, OxiModError::Index { .. }));
    }

    #[test]
    fn aggregation_domain_refines_operational_failure_to_aggregation() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::Aggregation,
            driver_error(command_kind(2)),
        );

        assert!(matches!(error, OxiModError::Aggregation { .. }));
    }

    #[test]
    fn bulk_write_domain_refines_operational_failure_to_bulk_write() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::BulkWrite,
            driver_error(command_kind(2)),
        );

        assert!(matches!(error, OxiModError::BulkWrite { .. }));
    }

    #[test]
    fn bulk_write_domain_refines_individual_write_failures_to_bulk_write() {
        let kind = ErrorKind::BulkWrite(mongodb::error::BulkWriteError::default());
        let error = classify_driver_error("ctx", OperationDomain::BulkWrite, driver_error(kind));

        assert!(matches!(error, OxiModError::BulkWrite { .. }));
    }

    // Fallback family: remaining current kinds — and, structurally, future
    // kinds of the #[non_exhaustive] driver enum — classify as Database.
    #[test]
    fn remaining_kinds_fall_back_to_database() {
        for kind in [
            ErrorKind::Shutdown,
            ErrorKind::SessionsNotSupported,
            ErrorKind::MissingResumeToken,
        ] {
            let error = classify_driver_error("ctx", OperationDomain::General, driver_error(kind));

            assert!(
                matches!(error, OxiModError::Database { .. }),
                "expected the fallback class to be Database, got: {error:?}"
            );
        }

        let custom = DriverError::custom("driver-level custom value".to_string());
        let error = classify_driver_error("ctx", OperationDomain::General, custom);

        assert!(matches!(error, OxiModError::Database { .. }));
    }

    // Source preservation: the original driver error remains available by
    // downcast, with server codes intact.
    #[test]
    fn classification_preserves_the_original_driver_error() {
        let error = classify_driver_error("ctx", OperationDomain::General, driver_error(io_kind()));

        let source = error
            .source()
            .and_then(|source| source.downcast_ref::<DriverError>())
            .expect("the driver error should remain available through source()");

        assert!(matches!(*source.kind, ErrorKind::Io(_)));
    }

    #[test]
    fn duplicate_key_code_remains_recoverable_through_source() {
        let error = classify_driver_error(
            "ctx",
            OperationDomain::General,
            driver_error(duplicate_key_kind()),
        );

        let source = error
            .source()
            .and_then(|source| source.downcast_ref::<DriverError>())
            .expect("the driver error should remain available through source()");

        let ErrorKind::Write(WriteFailure::WriteError(write_error)) = &*source.kind else {
            panic!("expected the preserved write failure, got: {source:?}");
        };

        assert_eq!(write_error.code, 11000);
    }

    // Context separation: `msg` flows into the variant's context unchanged
    // and never influences classification.
    #[test]
    fn context_message_is_preserved_verbatim() {
        let error = classify_driver_error(
            "Failed to insert document into MongoDB collection",
            OperationDomain::General,
            driver_error(duplicate_key_kind()),
        );

        assert!(
            error
                .to_string()
                .contains("Failed to insert document into MongoDB collection"),
            "expected the operation context in Display output, got: {error}"
        );
    }
}
