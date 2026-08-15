//! Explicit index drift inspection and conservative create-only
//! reconciliation.
//!
//! This module compares a collection model's declared `#[index(...)]`
//! specifications — the exact generated `IndexModel`s that OxiMod would
//! create — against the index metadata MongoDB reports for the model's
//! collection, and can recreate declarations that are genuinely missing.
//!
//! It is deliberately independent of the once-per-process index
//! establishment machinery used by `init_indexes*()` and save-triggered
//! initialization: inspection never reads or resets that state, and
//! reconciliation never mutates it.
//!
//! The safety contract is conservative by design:
//!
//! - inspection is read-only and never creates a collection or index;
//! - reconciliation creates **only** declarations classified as
//!   [`Missing`](crate::index_reconciliation::DeclaredIndexStatus::Missing);
//! - mismatched declarations and unmanaged raw-driver indexes are reported,
//!   never dropped, hidden, converted, or otherwise modified;
//! - no `dropIndexes`, `collMod`, or any destructive administrative command
//!   is ever issued by this module.
//!
//! Drift itself is report data, not an error:
//! [`OxiModError`](crate::error::oximod_error::OxiModError) is reserved
//! for failure to perform the inspection or creation operation.

mod compare;

use mongodb::{
    Collection, IndexModel,
    bson::doc,
    error::{Error as DriverError, ErrorKind},
    options::Collation,
};

use crate::error::classify::{OperationDomain, classify_driver_error};
use crate::error::oximod_error::OxiModError;

/// Point-in-time comparison of a model's declared indexes against the index
/// metadata reported by MongoDB.
///
/// Produced by generated `check_indexes()` / `check_indexes_from(&client)`
/// methods. The comparison is not transactional: another process may create,
/// drop, or change an index between inspection and any later action.
#[derive(Debug, Clone)]
pub struct IndexDriftReport {
    declared: Vec<DeclaredIndexStatus>,
    unmanaged: Vec<IndexModel>,
}

impl IndexDriftReport {
    /// Returns one status per declared `#[index(...)]` specification, in
    /// declaration order.
    pub fn declared(&self) -> &[DeclaredIndexStatus] {
        &self.declared
    }

    /// Returns server indexes unrelated to any declared specification, such
    /// as compound, partial, or other direct-driver indexes.
    ///
    /// The built-in `_id_` index is never reported. Unmanaged indexes are
    /// sorted by name and never affect [`IndexDriftReport::is_in_sync`]:
    /// direct-driver indexes are a supported escape hatch, not drift.
    pub fn unmanaged(&self) -> &[IndexModel] {
        &self.unmanaged
    }

    /// Returns whether every declared index is
    /// [`InSync`](DeclaredIndexStatus::InSync).
    ///
    /// Unmanaged direct-driver indexes do not make a report out of sync.
    pub fn is_in_sync(&self) -> bool {
        self.declared
            .iter()
            .all(|status| matches!(status, DeclaredIndexStatus::InSync { .. }))
    }

    /// Returns whether at least one declared index is missing or mismatched.
    pub fn has_drift(&self) -> bool {
        !self.is_in_sync()
    }

    /// Returns how many declared indexes are
    /// [`Missing`](DeclaredIndexStatus::Missing).
    pub fn missing_count(&self) -> usize {
        self.declared
            .iter()
            .filter(|status| matches!(status, DeclaredIndexStatus::Missing { .. }))
            .count()
    }

    /// Returns how many declared indexes are
    /// [`Mismatched`](DeclaredIndexStatus::Mismatched).
    pub fn mismatched_count(&self) -> usize {
        self.declared
            .iter()
            .filter(|status| matches!(status, DeclaredIndexStatus::Mismatched { .. }))
            .count()
    }
}

/// Drift status of one declared `#[index(...)]` specification.
///
/// Statuses are diagnostic data. Only [`Missing`](Self::Missing)
/// declarations are eligible for `create_missing_indexes*()`; a
/// [`Mismatched`](Self::Mismatched) declaration always requires an explicit
/// human decision because changing an existing index in place is a
/// destructive administrative action OxiMod refuses to take implicitly.
#[non_exhaustive]
#[derive(Debug, Clone)]
// The variants intentionally carry `IndexModel`s inline: the report surface
// promised to downstream tooling exposes the driver models directly, and
// reports are built once per explicit administrative call, never in hot
// paths.
#[allow(clippy::large_enum_variant)]
pub enum DeclaredIndexStatus {
    /// The server has an index that is semantically equivalent to the
    /// declaration, under the declaration's effective name.
    InSync {
        /// The declared `IndexModel` exactly as OxiMod would create it.
        expected: IndexModel,
        /// The matching server index as reported by `listIndexes`.
        actual: IndexModel,
    },

    /// No server index corresponds to the declaration; creating the declared
    /// index would not conflict with any inspected index.
    Missing {
        /// The declared `IndexModel` exactly as OxiMod would create it.
        expected: IndexModel,
    },

    /// One or more server indexes relate to the declaration — by effective
    /// name or by logical key — but differ semantically.
    Mismatched {
        /// The declared `IndexModel` exactly as OxiMod would create it.
        expected: IndexModel,
        /// Related server indexes and their differences, sorted by index
        /// name and key representation.
        candidates: Vec<IndexMismatchCandidate>,
    },
}

impl DeclaredIndexStatus {
    /// Returns the declared `IndexModel` this status describes.
    pub fn expected(&self) -> &IndexModel {
        match self {
            DeclaredIndexStatus::InSync { expected, .. }
            | DeclaredIndexStatus::Missing { expected }
            | DeclaredIndexStatus::Mismatched { expected, .. } => expected,
        }
    }
}

/// A server index related to a mismatched declaration, with the semantic
/// differences that prevent it from counting as in sync.
#[derive(Debug, Clone)]
pub struct IndexMismatchCandidate {
    actual: IndexModel,
    differences: Vec<String>,
}

impl IndexMismatchCandidate {
    /// Returns the related server index as reported by `listIndexes`.
    pub fn actual(&self) -> &IndexModel {
        &self.actual
    }

    /// Returns human-readable difference descriptions in a fixed property
    /// order (keys, name, then options).
    pub fn differences(&self) -> &[String] {
        &self.differences
    }
}

/// Outcome of one conservative create-only reconciliation pass.
///
/// Produced by generated `create_missing_indexes()` /
/// `create_missing_indexes_from(&client)` methods.
#[derive(Debug, Clone)]
pub struct IndexReconciliationReport {
    before: IndexDriftReport,
    attempted_creations: Vec<IndexModel>,
    after: IndexDriftReport,
}

impl IndexReconciliationReport {
    /// Returns the drift report observed before any creation was attempted.
    pub fn before(&self) -> &IndexDriftReport {
        &self.before
    }

    /// Returns the declared `IndexModel`s submitted to MongoDB, in
    /// declaration order.
    ///
    /// Only declarations classified [`Missing`](DeclaredIndexStatus::Missing)
    /// in the before-report are ever submitted. Empty when nothing was
    /// missing; in that case no `createIndexes` command was sent at all.
    pub fn attempted_creations(&self) -> &[IndexModel] {
        &self.attempted_creations
    }

    /// Returns the drift report observed after the creation attempt.
    pub fn after(&self) -> &IndexDriftReport {
        &self.after
    }

    /// Returns whether every declared index was in sync after
    /// reconciliation.
    ///
    /// `false` typically means mismatched declarations remain that require
    /// manual action.
    pub fn is_in_sync(&self) -> bool {
        self.after.is_in_sync()
    }
}

/// Index metadata inspected from an existing collection.
struct CollectionIndexState {
    indexes: Vec<IndexModel>,
    default_collation: Option<Collation>,
}

/// Returns whether a driver error is MongoDB's NamespaceNotFound rejection
/// (code 26), which `listIndexes` raises for a collection that does not
/// exist.
fn is_namespace_not_found(error: &DriverError) -> bool {
    matches!(*error.kind, ErrorKind::Command(ref command) if command.code == 26)
}

/// Inspects the collection's index metadata, returning `None` when the
/// collection does not exist.
///
/// The flow is read-only: collection existence and the collection's default
/// collation come from `listCollections`, and only an existing collection is
/// asked for `listIndexes`. A collection dropped between the two calls
/// surfaces as NamespaceNotFound and is treated as a missing collection
/// rather than an inspection failure.
async fn fetch_collection_index_state<T>(
    collection: &Collection<T>,
) -> Result<Option<CollectionIndexState>, OxiModError>
where
    T: Send + Sync,
{
    let collection_name = collection.name().to_string();
    let namespace = collection.namespace();
    let database = collection.client().database(&namespace.db);

    let mut specifications = database
        .list_collections()
        .filter(doc! { "name": &collection_name })
        .await
        .map_err(|error| {
            classify_driver_error(
                format!("Failed to list collection metadata for collection `{collection_name}`"),
                OperationDomain::Index,
                error,
            )
        })?;

    let mut specification = None;

    while specifications.advance().await.map_err(|error| {
        classify_driver_error(
            format!("Failed to advance collection metadata cursor for `{collection_name}`"),
            OperationDomain::Index,
            error,
        )
    })? {
        let candidate = specifications.deserialize_current().map_err(|error| {
            classify_driver_error(
                format!("Failed to deserialize collection metadata for `{collection_name}`"),
                OperationDomain::Index,
                error,
            )
        })?;

        if candidate.name == collection_name {
            specification = Some(candidate);
        }
    }

    let Some(specification) = specification else {
        return Ok(None);
    };

    let default_collation = specification.options.collation;

    let mut cursor = match collection.list_indexes().await {
        Ok(cursor) => cursor,
        Err(error) if is_namespace_not_found(&error) => return Ok(None),
        Err(error) => {
            return Err(classify_driver_error(
                format!("Failed to list indexes for collection `{collection_name}`"),
                OperationDomain::Index,
                error,
            ));
        }
    };

    let mut indexes = Vec::new();

    loop {
        match cursor.advance().await {
            Ok(true) => {}
            Ok(false) => break,
            Err(error) if is_namespace_not_found(&error) => return Ok(None),
            Err(error) => {
                return Err(classify_driver_error(
                    format!("Failed to advance index cursor for collection `{collection_name}`"),
                    OperationDomain::Index,
                    error,
                ));
            }
        }

        let index = cursor.deserialize_current().map_err(|error| {
            classify_driver_error(
                format!("Failed to deserialize index metadata for collection `{collection_name}`"),
                OperationDomain::Index,
                error,
            )
        })?;

        indexes.push(index);
    }

    Ok(Some(CollectionIndexState {
        indexes,
        default_collation,
    }))
}

/// Compares the declared indexes against the collection's current index
/// metadata.
///
/// This is macro-support infrastructure for the generated `check_indexes*()`
/// methods; call those methods instead. The inspection is read-only: a
/// collection that does not exist is not created, its declared indexes are
/// all reported [`Missing`](DeclaredIndexStatus::Missing), and a model with
/// zero declarations is in sync.
///
/// # Errors
///
/// Returns [`OxiModError`] when the metadata inspection itself fails:
/// `Connection` for connectivity failures, `Serialization` for BSON
/// decoding failures, and `Index` for server-side metadata rejections.
/// Drift is never an error.
#[doc(hidden)]
pub async fn check_indexes<T>(
    collection: &Collection<T>,
    declared: Vec<IndexModel>,
) -> Result<IndexDriftReport, OxiModError>
where
    T: Send + Sync,
{
    let state = fetch_collection_index_state(collection).await?;

    Ok(match state {
        Some(state) => compare::build_drift_report(
            &declared,
            Some(&state.indexes),
            state.default_collation.as_ref(),
        ),
        None => compare::build_drift_report(&declared, None, None),
    })
}

/// Creates only the declared indexes currently classified as
/// [`Missing`](DeclaredIndexStatus::Missing), then re-inspects.
///
/// This is macro-support infrastructure for the generated
/// `create_missing_indexes*()` methods; call those methods instead.
/// Mismatched declarations and unmanaged indexes are never touched, and no
/// `createIndexes` command is sent when nothing is missing — a model with
/// zero declared indexes never creates its collection. The
/// once-per-process establishment state used by `init_indexes*()` is
/// neither consulted nor modified.
///
/// # Errors
///
/// Returns [`OxiModError`] when inspection fails or MongoDB rejects the
/// creation (`Index` for server-side rejections such as a unique index over
/// duplicate data, `Connection` for connectivity failures). A failed
/// creation does not imply the server is unchanged; call the check method
/// again for current state.
#[doc(hidden)]
pub async fn create_missing_indexes<T>(
    collection: &Collection<T>,
    declared: Vec<IndexModel>,
) -> Result<IndexReconciliationReport, OxiModError>
where
    T: Send + Sync,
{
    let before = check_indexes(collection, declared.clone()).await?;

    let attempted_creations: Vec<IndexModel> = before
        .declared()
        .iter()
        .filter_map(|status| match status {
            DeclaredIndexStatus::Missing { expected } => Some(expected.clone()),
            _ => None,
        })
        .collect();

    if !attempted_creations.is_empty() {
        let collection_name = collection.name().to_string();

        collection
            .create_indexes(attempted_creations.clone())
            .await
            .map_err(|error| {
                classify_driver_error(
                    format!("Failed to create missing indexes for collection `{collection_name}`"),
                    OperationDomain::Index,
                    error,
                )
            })?;
    }

    let after = check_indexes(collection, declared).await?;

    Ok(IndexReconciliationReport {
        before,
        attempted_creations,
        after,
    })
}
