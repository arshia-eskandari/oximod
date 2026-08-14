//! Errors produced by invalid typed-query configuration.
//!
//! These errors are detected before a typed query is sent to MongoDB. They
//! cover invalid pagination values, limits outside MongoDB's supported range,
//! and query modifiers that cannot be used with multi-document or bulk-write
//! operations.

use std::fmt;

use thiserror::Error;

/// Represents an invalid typed-query configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QueryError {
    /// The supplied page number was zero.
    ///
    /// Page numbers are one-based.
    #[error("page number must be at least 1, got {page}")]
    InvalidPageNumber {
        /// The invalid page number.
        page: u64,
    },

    /// The supplied page size was zero.
    ///
    /// A page must request at least one result.
    #[error("page size must be at least 1, got {page_size}")]
    InvalidPageSize {
        /// The invalid page size.
        page_size: u64,
    },

    /// Calculating the pagination offset exceeded the range of `u64`.
    #[error("pagination offset overflowed for page {page} with page size {page_size}")]
    PaginationOverflow {
        /// The requested one-based page number.
        page: u64,

        /// The requested number of results per page.
        page_size: u64,
    },

    /// The supplied query limit exceeded MongoDB's signed 64-bit range.
    #[error("query limit {limit} exceeds MongoDB's supported range")]
    LimitOutOfRange {
        /// The unsupported query limit.
        limit: u64,
    },

    /// A multi-document or bulk-write operation was configured with an
    /// unsupported query modifier.
    #[error("{operation} does not support the `{modifier}` query modifier")]
    UnsupportedBulkWriteModifier {
        /// The operation being configured.
        operation: BulkWriteOperation,

        /// The unsupported query modifier.
        modifier: QueryModifier,
    },
}

/// A write operation named by query-preflight diagnostics.
///
/// This enum identifies which operation rejected an unsupported query
/// modifier in [`QueryError::UnsupportedBulkWriteModifier`]. It covers the
/// multi-document typed-query operations (`Query::update_all` and
/// `Query::delete_all`) and the operations queued through the
/// `BulkWrite` builder.
///
/// It names operations for preflight diagnostics only; it is **not** the
/// bulk-write builder's queued operation type and carries no operation
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkWriteOperation {
    /// Delete all documents matching the query.
    DeleteAll,

    /// Update all documents matching the query.
    UpdateAll,

    /// An update-one operation queued through the bulk-write builder.
    UpdateOne,

    /// An update-many operation queued through the bulk-write builder.
    UpdateMany,

    /// A replace-one operation queued through the bulk-write builder.
    ReplaceOne,

    /// A delete-one operation queued through the bulk-write builder.
    DeleteOne,

    /// A delete-many operation queued through the bulk-write builder.
    DeleteMany,
}

impl fmt::Display for BulkWriteOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeleteAll => formatter.write_str("delete_all"),
            Self::UpdateAll => formatter.write_str("update_all"),
            Self::UpdateOne => formatter.write_str("bulk_write update_one"),
            Self::UpdateMany => formatter.write_str("bulk_write update_many"),
            Self::ReplaceOne => formatter.write_str("bulk_write replace_one"),
            Self::DeleteOne => formatter.write_str("bulk_write delete_one"),
            Self::DeleteMany => formatter.write_str("bulk_write delete_many"),
        }
    }
}

/// A query modifier that is unsupported by bulk write operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryModifier {
    /// Query sorting.
    Sort,

    /// Query offset.
    Skip,

    /// Query result limit.
    Limit,

    /// One-based query pagination.
    Pagination,

    /// MongoDB array filters for filtered positional updates.
    ArrayFilters,
}

impl fmt::Display for QueryModifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sort => formatter.write_str("sort"),
            Self::Skip => formatter.write_str("skip"),
            Self::Limit => formatter.write_str("limit"),
            Self::Pagination => formatter.write_str("pagination"),
            Self::ArrayFilters => formatter.write_str("array_filters"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BulkWriteOperation, QueryError, QueryModifier};

    #[test]
    fn invalid_page_number_has_a_clear_message() {
        let error = QueryError::InvalidPageNumber { page: 0 };

        assert_eq!(error.to_string(), "page number must be at least 1, got 0");
    }

    #[test]
    fn invalid_page_size_has_a_clear_message() {
        let error = QueryError::InvalidPageSize { page_size: 0 };

        assert_eq!(error.to_string(), "page size must be at least 1, got 0");
    }

    #[test]
    fn pagination_overflow_preserves_inputs() {
        let error = QueryError::PaginationOverflow {
            page: u64::MAX,
            page_size: 2,
        };

        assert_eq!(
            error.to_string(),
            format!(
                "pagination offset overflowed for page {} with page size 2",
                u64::MAX
            ),
        );
    }

    #[test]
    fn limit_out_of_range_preserves_limit() {
        let error = QueryError::LimitOutOfRange { limit: u64::MAX };

        assert_eq!(
            error.to_string(),
            format!("query limit {} exceeds MongoDB's supported range", u64::MAX),
        );
    }

    #[test]
    fn unsupported_bulk_write_modifier_names_the_operation_and_modifier() {
        let error = QueryError::UnsupportedBulkWriteModifier {
            operation: BulkWriteOperation::UpdateAll,
            modifier: QueryModifier::Pagination,
        };

        assert_eq!(
            error.to_string(),
            "update_all does not support the `pagination` query modifier",
        );
    }

    #[test]
    fn bulk_write_operations_have_stable_display_names() {
        assert_eq!(BulkWriteOperation::DeleteAll.to_string(), "delete_all");
        assert_eq!(BulkWriteOperation::UpdateAll.to_string(), "update_all");
        assert_eq!(
            BulkWriteOperation::UpdateOne.to_string(),
            "bulk_write update_one"
        );
        assert_eq!(
            BulkWriteOperation::UpdateMany.to_string(),
            "bulk_write update_many"
        );
        assert_eq!(
            BulkWriteOperation::ReplaceOne.to_string(),
            "bulk_write replace_one"
        );
        assert_eq!(
            BulkWriteOperation::DeleteOne.to_string(),
            "bulk_write delete_one"
        );
        assert_eq!(
            BulkWriteOperation::DeleteMany.to_string(),
            "bulk_write delete_many"
        );
    }

    #[test]
    fn query_modifiers_have_stable_display_names() {
        assert_eq!(QueryModifier::Sort.to_string(), "sort");
        assert_eq!(QueryModifier::Skip.to_string(), "skip");
        assert_eq!(QueryModifier::Limit.to_string(), "limit");
        assert_eq!(QueryModifier::Pagination.to_string(), "pagination");
        assert_eq!(QueryModifier::ArrayFilters.to_string(), "array_filters");
    }
}
