//! Pagination extractor.

use arcana_core::PageRequest;
use serde::Deserialize;

/// Query parameters for pagination.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub size: Option<usize>,
}

impl From<PaginationQuery> for PageRequest {
    fn from(query: PaginationQuery) -> Self {
        PageRequest::new(
            query.page.unwrap_or(0),
            query.size.unwrap_or(PageRequest::DEFAULT_SIZE),
        )
    }
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(0),
            size: Some(PageRequest::DEFAULT_SIZE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_query_default_uses_page_0_and_default_size() {
        let q = PaginationQuery::default();
        assert_eq!(q.page, Some(0));
        assert_eq!(q.size, Some(PageRequest::DEFAULT_SIZE));
    }

    #[test]
    fn pagination_query_with_values_converts_to_page_request() {
        let q = PaginationQuery {
            page: Some(2),
            size: Some(25),
        };
        let pr: PageRequest = q.into();
        assert_eq!(pr.page, 2);
        assert_eq!(pr.size, 25);
    }

    #[test]
    fn pagination_query_none_values_use_defaults() {
        let q = PaginationQuery {
            page: None,
            size: None,
        };
        let pr: PageRequest = q.into();
        assert_eq!(pr.page, 0);
        assert_eq!(pr.size, PageRequest::DEFAULT_SIZE);
    }

    #[test]
    fn pagination_query_clone_and_debug() {
        let q = PaginationQuery { page: Some(1), size: Some(10) };
        let cloned = q.clone();
        assert_eq!(cloned.page, Some(1));
        assert!(format!("{:?}", cloned).contains("PaginationQuery"));
    }
}
