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
