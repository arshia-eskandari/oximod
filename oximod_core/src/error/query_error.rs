use std::fmt;
use thiserror::Error;

/// Represents errors caused by invalid typed-query configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QueryError {
    /// Page numbers are one-based.
    #[error("page number must be at least 1, got {page}")]
    InvalidPageNumber { page: u64 },

    /// A page must request at least one result.
    #[error("page size must be at least 1, got {page_size}")]
    InvalidPageSize { page_size: u64 },

    /// Calculating the pagination offset exceeded `u64`.
    #[error("pagination offset overflowed for page {page} with page size {page_size}")]
    PaginationOverflow { page: u64, page_size: u64 },

    /// MongoDB represents limits using a signed 64-bit integer.
    #[error("query limit {limit} exceeds MongoDB's supported range")]
    LimitOutOfRange { limit: u64 },

    /// A bulk write was configured with an unsupported query modifier.
    #[error("{operation} does not support the `{modifier}` query modifier")]
    UnsupportedBulkWriteModifier {
        /// The bulk operation being configured.
        operation: BulkWriteOperation,

        /// The unsupported query modifier.
        modifier: QueryModifier,
    },
}

/// A typed-query operation that may modify multiple documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkWriteOperation {
    /// Delete all documents matching the query.
    DeleteAll,

    /// Update all documents matching the query.
    UpdateAll,
}

impl fmt::Display for BulkWriteOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeleteAll => f.write_str("delete_all"),
            Self::UpdateAll => f.write_str("update_all"),
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
}

impl fmt::Display for QueryModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sort => f.write_str("sort"),
            Self::Skip => f.write_str("skip"),
            Self::Limit => f.write_str("limit"),
            Self::Pagination => f.write_str("pagination"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QueryError;

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
            )
        );
    }

    #[test]
    fn limit_out_of_range_preserves_limit() {
        let error = QueryError::LimitOutOfRange { limit: u64::MAX };

        assert_eq!(
            error.to_string(),
            format!("query limit {} exceeds MongoDB's supported range", u64::MAX)
        );
    }
}
