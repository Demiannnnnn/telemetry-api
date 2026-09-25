//! # Models Module
//!
//! Shared models and Data Transfer Objects (DTOs) for standard API responses and pagination.

use serde::{Deserialize, Serialize};

/// Standard single-item JSON response envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SingleResponse<T> {
    /// Enclosed resource data.
    pub data: T,
}

impl<T> SingleResponse<T> {
    /// Creates a new single resource envelope.
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Standard paginated collection JSON response envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionResponse<T> {
    /// List of resources for the requested page.
    pub data: Vec<T>,
    /// Pagination metadata.
    pub pagination: PaginationMeta,
}

impl<T> CollectionResponse<T> {
    /// Creates a new collection response with pagination metadata.
    pub fn new(data: Vec<T>, pagination: PaginationMeta) -> Self {
        Self { data, pagination }
    }
}

/// Query parameters for paginated endpoints.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PaginationParams {
    /// Page number (1-based index).
    pub page: Option<u32>,
    /// Number of items per page (1 to 100).
    pub per_page: Option<u32>,
}

impl PaginationParams {
    /// Returns the validated 1-based page number (defaults to 1).
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Returns the validated items per page (defaults to 20, clamped between 1 and 100).
    pub fn per_page(&self) -> u32 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    /// Calculates SQL OFFSET based on validated page and per_page.
    pub fn offset(&self) -> i64 {
        ((self.page() - 1) * self.per_page()) as i64
    }

    /// Returns SQL LIMIT based on validated per_page.
    pub fn limit(&self) -> i64 {
        self.per_page() as i64
    }
}

/// Pagination metadata returned with collection responses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaginationMeta {
    /// Total count of matching records across all pages.
    pub total: i64,
    /// Current 1-based page number.
    pub page: u32,
    /// Number of records per page.
    pub per_page: u32,
    /// Total number of available pages.
    pub total_pages: u32,
}

impl PaginationMeta {
    /// Computes pagination metadata from total count, page, and per_page.
    pub fn new(total: i64, page: u32, per_page: u32) -> Self {
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        let total_pages = if total <= 0 {
            0
        } else {
            ((total as f64) / (per_page as f64)).ceil() as u32
        };

        Self {
            total: total.max(0),
            page,
            per_page,
            total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults_and_limits() {
        let default_params = PaginationParams::default();
        assert_eq!(default_params.page(), 1);
        assert_eq!(default_params.per_page(), 20);
        assert_eq!(default_params.offset(), 0);
        assert_eq!(default_params.limit(), 20);

        let custom_params = PaginationParams {
            page: Some(3),
            per_page: Some(50),
        };
        assert_eq!(custom_params.page(), 3);
        assert_eq!(custom_params.per_page(), 50);
        assert_eq!(custom_params.offset(), 100);
        assert_eq!(custom_params.limit(), 50);

        // Clamping check: per_page > 100 is clamped to 100, page 0 becomes 1
        let clamped_params = PaginationParams {
            page: Some(0),
            per_page: Some(500),
        };
        assert_eq!(clamped_params.page(), 1);
        assert_eq!(clamped_params.per_page(), 100);
        assert_eq!(clamped_params.offset(), 0);
        assert_eq!(clamped_params.limit(), 100);
    }

    #[test]
    fn test_pagination_meta_calculation() {
        let meta = PaginationMeta::new(105, 1, 20);
        assert_eq!(meta.total, 105);
        assert_eq!(meta.page, 1);
        assert_eq!(meta.per_page, 20);
        assert_eq!(meta.total_pages, 6);

        let empty_meta = PaginationMeta::new(0, 1, 20);
        assert_eq!(empty_meta.total, 0);
        assert_eq!(empty_meta.total_pages, 0);
    }

    #[test]
    fn test_response_envelopes_serialization() {
        let single = SingleResponse::new("hello".to_string());
        let serialized = serde_json::to_string(&single).expect("serialize single");
        assert_eq!(serialized, r#"{"data":"hello"}"#);

        let collection = CollectionResponse::new(vec![1, 2, 3], PaginationMeta::new(3, 1, 10));
        let serialized = serde_json::to_string(&collection).expect("serialize collection");
        assert!(serialized.contains(r#""data":[1,2,3]"#));
        assert!(serialized.contains(r#""total":3"#));
    }
}
