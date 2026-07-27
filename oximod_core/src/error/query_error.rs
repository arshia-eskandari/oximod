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
